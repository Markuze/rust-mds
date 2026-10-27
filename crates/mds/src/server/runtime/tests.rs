//! Runtime regressions use the production engine and explicit, delayed storage
//! callbacks. The command recorder is not native RADOS durability evidence.

mod d5;
mod d6;
mod recovery;

use super::*;
use crate::persistence::EventKind;
use denc::{EntityAddr, EntityAddrVec, Message, RawMessage};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Clone)]
struct Peer {
    addr: EntityAddr,
    identity: (String, Bytes),
    output: Arc<Mutex<Vec<RawMessage>>>,
    connection: u64,
}
impl Peer {
    fn new(nonce: u32) -> Self {
        Self {
            addr: EntityAddr::new(
                denc::AddrType::LEGACY,
                "127.0.0.1:4100".parse().unwrap(),
                nonce,
            ),
            identity: ("client.test".into(), Bytes::new()),
            output: Default::default(),
            connection: u64::from(nonce),
        }
    }
    fn clear(&self) {
        self.output.lock().unwrap().clear();
    }
}
impl ClientLink for Peer {
    fn connection_id(&self) -> u64 {
        self.connection
    }
    fn send_raw(&self, raw: RawMessage) -> bool {
        self.output.lock().unwrap().push(raw);
        true
    }
    fn features(&self) -> u64 {
        denc::features::ALL
    }
    fn peer_addr(&self) -> EntityAddr {
        self.addr
    }
    fn authenticated_identity(&self) -> Option<(String, Bytes)> {
        Some(self.identity.clone())
    }
}

struct Fixture {
    server: Server,
    sessions: SessionMap,
    commands: mpsc::Receiver<Command>,
    next_incarnation: u64,
    durable_keys: HashMap<u64, SessionKey>,
    events: Vec<&'static str>,
    initial_trim: u64,
}
impl Fixture {
    fn new() -> Self {
        let mut fixture = Self::unready();
        assert!(!fixture.server.try_finish_recovery(&mut fixture.sessions));
        assert!(fixture.server.recovery_structural_done());
        fixture.drain();
        assert!(fixture.server.try_finish_recovery(&mut fixture.sessions));
        fixture.server.pause_data_work(false);
        fixture
    }
    fn unready() -> Self {
        Self::unready_config(EngineConfig::default())
    }
    fn unready_config(config: EngineConfig) -> Self {
        let state = Engine::new(config.clone(), crate::journal::SeedJournal::new())
            .checkpoint()
            .unwrap();
        let owner = OwnerToken {
            shard_epoch: 1,
            rank: 0,
            incarnation: 1,
            gid: 1,
        };
        let rank_owner = store::RankOwner {
            rank: 0,
            incarnation: 1,
            gid: 1,
        };
        let loaded = Loaded {
            identity: store::StoreIdentity {
                cluster_fsid: [7; 16],
                filesystem_id: 3,
                metadata_pool: 6,
            },
            owner,
            tail: state.covered_seq,
            checkpoint: store::CheckpointDescriptor {
                generation: 1,
                covered_seq: state.covered_seq,
                manifest_name: "fixture".into(),
                manifest_checksum: 0,
            },
            state,
            records: vec![],
            sessions: store::RankSessionState {
                control: store::SessionsInspection {
                    required_semantics: engine::RecoverySemantics::RetainedTargets3,
                    owner: rank_owner,
                    writer: store::WriterIdentity {
                        rank_owner,
                        addresses: EntityAddrVec(vec![Peer::new(1).addr]),
                    },
                    predecessor: None,
                    revision: 1,
                    next_incarnation: 1,
                },
                records: vec![],
            },
            initial_range: config.ino_lease,
            barrier: 0,
        };
        let (tx, commands) = mpsc::channel(4096);
        let server = Server::from_loaded(
            "a".into(),
            1,
            1,
            loaded,
            config,
            &store::Limits::default(),
            tx,
            1,
            16,
            Duration::from_secs(30),
        )
        .unwrap();
        let mut fixture = Self {
            server,
            sessions: SessionMap::default(),
            commands,
            next_incarnation: 1,
            durable_keys: HashMap::new(),
            events: vec![],
            initial_trim: 1,
        };
        fixture.drain();
        fixture
    }
    fn send<M: Message>(&mut self, client: u64, peer: &Peer, msg: M) {
        crate::daemon::dispatch_client_raw(
            &mut self.server,
            &mut self.sessions,
            client,
            Box::new(peer.clone()),
            vec![],
            msg.to_raw(peer.features()),
        );
    }
    fn complete(&mut self, command: Command) {
        let event = match command.kind {
            CommandKind::TrimInitial => EventKind::InitialTrimmed(self.initial_trim),
            CommandKind::Open(identity) => {
                let key = *self.durable_keys.entry(identity.client).or_insert_with(|| {
                    let key = SessionKey {
                        client: identity.client,
                        incarnation: self.next_incarnation,
                    };
                    self.next_incarnation += 1;
                    key
                });
                self.events.push("Open");
                EventKind::Opened(SessionRecord {
                    key,
                    identity,
                    lifecycle: SessionLifecycle::Open,
                    cleanup_seq: None,
                })
            }
            CommandKind::Append { seq, body } => {
                store::record::decode_record(&body).unwrap();
                self.events.push("Commit");
                EventKind::Committed(seq)
            }
            CommandKind::Closing(key) => {
                self.events.push("Closing");
                EventKind::Closing(key)
            }
            CommandKind::Closed { key, cleanup_seq } => {
                self.durable_keys.remove(&key.client);
                self.events.push("Closed");
                EventKind::Closed { key, cleanup_seq }
            }
            CommandKind::Checkpoint(state) => EventKind::Checkpoint(store::CheckpointDescriptor {
                generation: 2,
                covered_seq: state.covered_seq,
                manifest_name: "fixture2".into(),
                manifest_checksum: 0,
            }),
        };
        self.server
            .on_storage_event(&mut self.sessions, command.owner, command.operation, event);
        assert!(
            self.server.fatal_error().is_none(),
            "{:?}",
            self.server.fatal_error()
        );
        assert!(self.slots() <= 1024);
        assert!(self.server.runtime.as_ref().unwrap().session_slots() <= 1024);
    }
    fn drain(&mut self) {
        for _ in 0..4096 {
            self.server.pump(&mut self.sessions, 32);
            if let Ok(command) = self.commands.try_recv() {
                self.complete(command);
            } else if !self.server.has_work() {
                return;
            }
        }
        panic!("runtime did not drain");
    }
    fn open(&mut self, client: u64, peer: &Peer) {
        self.send(
            client,
            peer,
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
        );
        self.drain();
        assert!(self.sessions.get(client).unwrap().is_open());
    }
    fn slots(&self) -> usize {
        self.sessions.len()
            + self
                .server
                .runtime
                .as_ref()
                .unwrap()
                .opening
                .keys()
                .filter(|client| self.sessions.get(**client).is_none())
                .count()
    }
    fn create(&mut self, client: u64, peer: &Peer) -> u64 {
        let mut request = proto::MClientRequest::new(1, MdsOp::CREATE);
        request.path = WireFilePath::new(1, "file");
        request.head.flags = proto::request::request_flags::WANT_DENTRY;
        request.head.args = proto::request::OpenArgs {
            flags: proto::open_flags::CREAT | proto::open_flags::RDWR,
            mode: 0o644,
            pool: 2,
            ..Default::default()
        }
        .to_args();
        self.send(client, peer, request);
        self.drain();
        peer.output
            .lock()
            .unwrap()
            .iter()
            .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_REPLY)
            .map(|raw| MClientReply::from_raw(raw.clone()).unwrap())
            .find(|reply| reply.tid == 1)
            .unwrap()
            .trace
            .target
            .unwrap()
            .ino
    }

    fn cold_restore(&mut self) {
        let state = self.server.engine.checkpoint().unwrap();
        let config = EngineConfig::default();
        let owner = self.server.owner();
        let rows = self
            .server
            .runtime
            .as_ref()
            .unwrap()
            .rows
            .values()
            .cloned()
            .collect();
        let rank = store::RankOwner {
            rank: 0,
            incarnation: owner.incarnation,
            gid: owner.gid,
        };
        let loaded = Loaded {
            identity: store::StoreIdentity {
                cluster_fsid: [7; 16],
                filesystem_id: 3,
                metadata_pool: 6,
            },
            owner,
            tail: state.covered_seq,
            checkpoint: store::CheckpointDescriptor {
                generation: 1,
                covered_seq: state.covered_seq,
                manifest_name: "restored-fixture".into(),
                manifest_checksum: 0,
            },
            state,
            records: vec![],
            sessions: store::RankSessionState {
                control: store::SessionsInspection {
                    required_semantics: engine::RecoverySemantics::RetainedTargets3,
                    owner: rank,
                    writer: store::WriterIdentity {
                        rank_owner: rank,
                        addresses: EntityAddrVec(vec![Peer::new(1).addr]),
                    },
                    predecessor: None,
                    revision: 1,
                    next_incarnation: self.next_incarnation,
                },
                records: rows,
            },
            initial_range: config.ino_lease,
            barrier: 9,
        };
        self.initial_trim = loaded.tail;
        let (tx, commands) = mpsc::channel(4096);
        self.server = Server::from_loaded(
            "a".into(),
            2,
            1,
            loaded,
            config,
            &store::Limits::default(),
            tx,
            1,
            16,
            Duration::from_secs(30),
        )
        .unwrap();
        self.sessions = SessionMap::default();
        self.commands = commands;
        self.drain();
        assert!(!self.server.try_finish_recovery(&mut self.sessions));
        self.server
            .confirm_reconnect(Instant::now(), Duration::from_secs(45));
    }
    fn flush(&self, client: u64, ino: u64) -> MClientCaps {
        let cap = self.server.engine.cap(ino, client).unwrap();
        MClientCaps {
            op: CapOp::FLUSH,
            ino,
            cap_id: cap.cap_id,
            seq: cap.last_sent,
            issue_seq: cap.last_issue,
            caps: cap.issued(),
            wanted: cap.wanted(),
            dirty: proto::types::cap::FILE_WR,
            tid: 99,
            size: 1234,
            ..Default::default()
        }
    }
}

#[test]
fn g3_e2_empty_checked_restore_needs_fresh_admitted_marker_without_commit_callback() {
    let mut f = Fixture::unready();
    let tail = f.server.engine.submitted_seq();
    assert!(
        !f.server.try_finish_recovery(&mut f.sessions),
        "structural finish alone is not readiness"
    );
    assert!(!f.server.recovered());
    assert!(
        f.server.has_work(),
        "the release marker must schedule without a client, Tick or WAL callback"
    );
    f.server.pump(&mut f.sessions, 1);
    assert!(f.server.try_finish_recovery(&mut f.sessions));
    assert!(f.server.recovered());
    assert_eq!(f.server.engine.submitted_seq(), tail);
    assert!(f.commands.try_recv().is_err());
    let permit = f
        .server
        .engine
        .journal_mut()
        .reserve()
        .expect("the marker returned its unused permit");
    f.server.engine.journal_mut().release(Some(permit));
    f.server.pause_data_work(false);
    let peer = Peer::new(10);
    f.open(42, &peer);
    let _ino = f.create(42, &peer);
}

#[test]
fn g3_e2_delayed_marker_permit_keeps_the_obligation_without_a_storage_wakeup() {
    let mut f = Fixture::unready();
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
    let tail = f.server.engine.submitted_seq();
    let held = f.server.engine.journal_mut().reserve().unwrap();
    f.server.pump(&mut f.sessions, 1);
    assert!(!f.server.recovered());
    assert!(!f.server.has_work(), "a held permit is not runnable work");
    assert!(f.commands.try_recv().is_err());
    f.server.engine.journal_mut().release(Some(held));
    assert!(f.server.has_work());
    f.server.pump(&mut f.sessions, 1);
    assert!(
        f.server.try_finish_recovery(&mut f.sessions),
        "a returned permit needs no unrelated checkpoint callback"
    );
    assert_eq!(f.server.engine.submitted_seq(), tail);
    assert!(f.commands.try_recv().is_err());
}

#[test]
fn g3_e2_clientless_retained_size_job_runs_before_marker_and_service_readiness() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.open(42, &peer);
    let ino = f.create(42, &peer);
    let (tx, mut evictions) = mpsc::channel(4);
    f.server.set_evict_channel(tx);
    f.server.request_eviction(&mut f.sessions, 42);
    assert!(
        f.server
            .finish_eviction(&mut f.sessions, evictions.try_recv().unwrap(), 9)
    );
    f.drain();
    f.cold_restore();
    assert!(f.server.recovery_structural_done());
    assert!(!f.server.recovered());
    f.server.set_runtime_permissions(false, true, true);
    let job = f
        .server
        .ready_jobs()
        .into_iter()
        .find(|job| matches!(job.kind, engine::LifecycleKind::RecoverSize { .. }))
        .unwrap();
    assert_eq!(
        f.server.runtime.as_ref().unwrap().recovery_drain,
        RecoveryDrain::Controls
    );
    assert!(
        !f.server.has_work(),
        "waiting for a required job must not spin on a release marker"
    );
    f.server.lifecycle_completed(
        job.id,
        engine::LifecycleResult::Recovered {
            size: 1234,
            mtime: engine::Timestamp::new(42, 0),
        },
    );
    f.server.pump(&mut f.sessions, 1);
    assert!(!f.server.recovered());
    assert_eq!(
        f.server.runtime.as_ref().unwrap().recovery_drain,
        RecoveryDrain::Controls
    );
    assert!(
        f.server.has_job(job.id),
        "job retirement must commit before the marker"
    );
    f.drain();
    assert!(f.server.try_finish_recovery(&mut f.sessions));
    assert!(!f.server.has_job(job.id));
    assert_eq!(f.server.engine.inode(ino).unwrap().size, 1234);
}

#[test]
fn g3_early_retry_starts_silent_durable_attachment_before_reconnect() {
    let mut f = Fixture::new();
    let old = Peer::new(10);
    f.open(42, &old);
    let _ino = f.create(42, &old);
    f.cold_restore();
    let new = Peer::new(20);
    let mut retry = proto::MClientRequest::new(1, MdsOp::CREATE);
    retry.path = WireFilePath::new(1, "file");
    f.send(42, &new, retry);
    let command = f
        .commands
        .try_recv()
        .expect("early authenticated retry must start durable attachment");
    assert!(matches!(command.kind, CommandKind::Open(_)));
    f.complete(command);
    assert!(
        new.output.lock().unwrap().is_empty(),
        "recovery attachment must not send OPEN"
    );
    assert_eq!(f.server.gather_len(), 1);
}

#[test]
fn g3_reconnect_reconstructs_without_open_or_nonempty_finalization() {
    let mut f = Fixture::new();
    let old = Peer::new(10);
    f.open(42, &old);
    let ino = f.create(42, &old);
    let cap = f.server.engine.cap(ino, 42).unwrap().clone();
    f.cold_restore();
    let new = Peer::new(20);
    f.send(
        42,
        &new,
        proto::MClientReconnect {
            caps: [(
                ino,
                proto::CapReconnect {
                    cap_id: cap.cap_id,
                    issued: cap.issued(),
                    wanted: cap.wanted(),
                    snaprealm: 1,
                    ..Default::default()
                },
            )]
            .into(),
            ..Default::default()
        },
    );
    f.drain();
    assert!(
        f.server.engine.cap(ino, 42).is_some(),
        "production dispatch must apply the reconnect chunk"
    );
    assert!(
        new.output
            .lock()
            .unwrap()
            .iter()
            .all(|raw| raw.msg_type != proto::msg_type::CLIENT_SESSION)
    );
    assert_eq!(f.server.gather_len(), 0);
    assert!(
        !f.server.try_finish_recovery(&mut f.sessions),
        "E2 finalization is not implemented in this package"
    );
}

#[test]
fn g2_r1_changed_peer_cannot_mutate_before_durable_attachment() {
    let mut f = Fixture::new();
    let a = Peer::new(10);
    let b = Peer::new(20);
    f.open(42, &a);
    let ino = f.create(42, &a);
    a.clear();
    let before = f.server.engine.inode(ino).unwrap().clone();
    let tail = f.server.engine.submitted_seq();
    let mut mkdir = proto::MClientRequest::new(2, MdsOp::MKDIR);
    mkdir.path = WireFilePath::new(1, "unattached");
    f.send(42, &b, mkdir.clone());
    f.send(42, &b, f.flush(42, ino));
    assert_eq!(
        f.server.engine.submitted_seq(),
        tail,
        "unattached B must not enter the engine"
    );
    assert_eq!(f.server.engine.inode(ino).unwrap(), &before);
    assert!(f.server.runtime.as_ref().unwrap().normal.is_empty());
    assert!(f.server.runtime.as_ref().unwrap().control.is_empty());
    assert!(f.commands.try_recv().is_err());
    assert!(a.output.lock().unwrap().is_empty());
    f.send(42, &b, MClientSession::new(SessionOp::REQUEST_OPEN, 0));
    let attach = f.commands.try_recv().unwrap();
    assert!(
        matches!(&attach.kind, CommandKind::Open(identity) if identity.peer_addresses.0.contains(&a.addr) && identity.peer_addresses.0.contains(&b.addr))
    );
    f.send(42, &b, mkdir.clone());
    f.send(42, &b, f.flush(42, ino));
    assert_eq!(f.server.engine.submitted_seq(), tail);
    assert!(a.output.lock().unwrap().is_empty());
    assert!(b.output.lock().unwrap().is_empty());
    f.complete(attach);
    assert!(f.server.fence_addresses(42).contains(&a.addr));
    assert!(f.server.fence_addresses(42).contains(&b.addr));
    f.send(42, &b, mkdir);
    f.drain();
    assert!(f.server.engine.submitted_seq() > tail);
    assert_eq!(f.sessions.get(42).unwrap().addr, b.addr);
    assert!(a.output.lock().unwrap().is_empty());
}

#[test]
fn g2_r1_same_address_reset_is_allowed_but_changed_credentials_wait() {
    let mut f = Fixture::new();
    let a = Peer::new(10);
    f.open(42, &a);
    let ino = f.create(42, &a);
    a.clear();
    let reset = Peer::new(10);
    let mut request = proto::MClientRequest::new(2, MdsOp::MKDIR);
    request.path = WireFilePath::new(1, "same-address-reset");
    let tail = f.server.engine.submitted_seq();
    f.send(42, &reset, request);
    f.drain();
    assert!(f.server.engine.submitted_seq() > tail);
    a.clear();
    let mut changed = Peer::new(10);
    changed.identity.1 = Bytes::from_static(b"new credential context");
    let tail = f.server.engine.submitted_seq();
    let before = f.server.engine.inode(ino).unwrap().clone();
    let mut request = proto::MClientRequest::new(3, MdsOp::MKDIR);
    request.path = WireFilePath::new(1, "changed-auth");
    f.send(42, &changed, request.clone());
    f.send(42, &changed, f.flush(42, ino));
    f.send(
        42,
        &changed,
        MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 77),
    );
    f.send(
        42,
        &changed,
        MClientSession::new(
            SessionOp::REQUEST_CLOSE,
            f.sessions.get(42).unwrap().push_seq(),
        ),
    );
    assert_eq!(f.server.engine.submitted_seq(), tail);
    assert_eq!(f.server.engine.inode(ino).unwrap(), &before);
    assert!(f.sessions.get(42).unwrap().is_open());
    assert!(a.output.lock().unwrap().is_empty());
    f.send(
        42,
        &changed,
        MClientSession::new(SessionOp::REQUEST_OPEN, 0),
    );
    let attach = f.commands.try_recv().unwrap();
    assert!(
        matches!(&attach.kind, CommandKind::Open(identity) if identity.authorization_context == changed.identity.1)
    );
    f.send(42, &changed, request.clone());
    assert_eq!(f.server.engine.submitted_seq(), tail);
    f.complete(attach);
    f.send(42, &changed, request);
    f.drain();
    assert!(f.server.engine.submitted_seq() > tail);
    let tail = f.server.engine.submitted_seq();
    f.send(42, &a, f.flush(42, ino));
    assert_eq!(f.server.engine.submitted_seq(), tail);
}

#[test]
fn g2_r1_stale_attachment_generation_cannot_publish_or_keep_reservation() {
    for was_live in [false, true] {
        let mut f = Fixture::new();
        let a = Peer::new(10);
        let b = Peer::new(20);
        if was_live {
            f.open(42, &a);
        }
        a.clear();
        f.send(42, &b, MClientSession::new(SessionOp::REQUEST_OPEN, 0));
        let command = f.commands.try_recv().unwrap();
        let CommandKind::Open(identity) = command.kind else {
            panic!("expected attachment");
        };
        let key = f.durable_keys.get(&42).copied().unwrap_or(SessionKey {
            client: 42,
            incarnation: 1,
        });
        if let Some(s) = f.sessions.get_mut(42) {
            s.generation += 1;
        } else {
            // Simulate an unrelated runtime install before this new Open's callback.
            f.sessions.handle(
                42,
                Box::new(a.clone()),
                MClientSession::new(SessionOp::REQUEST_OPEN, 0),
                vec![],
            );
            a.clear();
        }
        f.server.on_storage_event(
            &mut f.sessions,
            command.owner,
            command.operation,
            EventKind::Opened(SessionRecord {
                key,
                identity,
                lifecycle: SessionLifecycle::Open,
                cleanup_seq: None,
            }),
        );
        assert!(f.server.fatal_error().is_some());
        assert_eq!(f.sessions.get(42).unwrap().addr, a.addr);
        assert!(a.output.lock().unwrap().is_empty());
        assert!(b.output.lock().unwrap().is_empty());
        let r = f.server.runtime.as_ref().unwrap();
        assert!(r.opening.is_empty());
        assert_eq!(r.opening_bytes, 0);
        assert_eq!(r.session_slots(), usize::from(was_live));
    }
}

#[test]
fn g2_r2_delayed_flush_invalidates_close_until_current_sequence_retry() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.open(42, &peer);
    let ino = f.create(42, &peer);
    let mut request = proto::MClientRequest::new(2, MdsOp::MKDIR);
    request.path = WireFilePath::new(1, "delayed");
    f.send(42, &peer, request);
    let delayed = f.commands.try_recv().unwrap();
    assert!(matches!(delayed.kind, CommandKind::Append { .. }));
    f.send(42, &peer, f.flush(42, ino));
    let seq = f.sessions.get(42).unwrap().push_seq();
    f.send(
        42,
        &peer,
        MClientSession::new(SessionOp::REQUEST_CLOSE, seq),
    );
    f.complete(delayed);
    let flush = f.commands.try_recv().unwrap();
    assert!(matches!(flush.kind, CommandKind::Append { .. }));
    f.complete(flush);
    assert!(f.sessions.get(42).unwrap().push_seq() > seq);
    assert!(
        f.commands.try_recv().is_err(),
        "obsolete CLOSE must not persist Closing"
    );
    assert!(f.server.engine.cap(ino, 42).is_some());
    f.send(
        42,
        &peer,
        MClientSession::new(SessionOp::REQUEST_CLOSE, seq),
    );
    assert!(
        f.commands.try_recv().is_err(),
        "obsolete retry must remain non-authoritative"
    );
    let seq = f.sessions.get(42).unwrap().push_seq();
    f.events.clear();
    f.send(
        42,
        &peer,
        MClientSession::new(SessionOp::REQUEST_CLOSE, seq),
    );
    f.drain();
    assert!(f.sessions.get(42).is_none());
    assert_eq!(f.events.first(), Some(&"Closing"));
    assert_eq!(f.events.last(), Some(&"Closed"));
    assert!(
        peer.output
            .lock()
            .unwrap()
            .iter()
            .any(|raw| raw.msg_type == proto::msg_type::CLIENT_SESSION
                && MClientSession::from_raw(raw.clone()).unwrap().op == SessionOp::CLOSE)
    );
}

#[test]
fn g2_r2_paused_counted_output_prevents_close_publication() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.open(42, &peer);
    peer.clear();
    f.sessions.pause_output(true);
    f.sessions
        .get_mut(42)
        .unwrap()
        .send(&MClientCaps::default());
    f.send(42, &peer, MClientSession::new(SessionOp::REQUEST_CLOSE, 0));
    assert!(
        f.commands.try_recv().is_err(),
        "queued counted output is not acknowledged by CLOSE(0)"
    );
    assert!(
        !f.server.has_work(),
        "waiting for output/current CLOSE must not spin"
    );
    f.sessions.pause_output(false);
    f.sessions.drain_output(32);
    assert_eq!(f.sessions.get(42).unwrap().push_seq(), 1);
    f.send(42, &peer, MClientSession::new(SessionOp::REQUEST_CLOSE, 1));
    f.drain();
    assert!(f.sessions.get(42).is_none());
    let out = peer.output.lock().unwrap();
    assert_eq!(out[0].msg_type, proto::msg_type::CLIENT_CAPS);
    assert_eq!(
        MClientSession::from_raw(out[1].clone()).unwrap().op,
        SessionOp::CLOSE
    );
}

#[test]
fn g2_r2_noncounted_terminal_backlog_keeps_order_after_durable_close() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.open(42, &peer);
    peer.clear();
    f.sessions.pause_output(true);
    f.sessions
        .get_mut(42)
        .unwrap()
        .send(&MClientSession::renewcaps(77));
    f.send(42, &peer, MClientSession::new(SessionOp::REQUEST_CLOSE, 0));
    f.drain();
    assert!(f.sessions.get(42).is_none());
    assert!(peer.output.lock().unwrap().is_empty());
    f.sessions.pause_output(false);
    f.sessions.drain_output(32);
    let out = peer.output.lock().unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(
        MClientSession::from_raw(out[0].clone()).unwrap().op,
        SessionOp::RENEWCAPS
    );
    assert_eq!(
        MClientSession::from_raw(out[1].clone()).unwrap().op,
        SessionOp::CLOSE
    );
}

#[test]
fn g2_r2_verified_fencing_can_finish_a_close_waiting_for_sequence_retry() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.open(42, &peer);
    f.sessions.pause_output(true);
    f.sessions
        .get_mut(42)
        .unwrap()
        .send(&MClientCaps::default());
    f.send(42, &peer, MClientSession::new(SessionOp::REQUEST_CLOSE, 0));
    assert!(f.commands.try_recv().is_err());
    let (tx, mut rx) = mpsc::channel(4);
    f.server.set_evict_channel(tx);
    f.server.request_eviction(&mut f.sessions, 42);
    let request = rx.try_recv().unwrap();
    assert!(f.server.finish_eviction(&mut f.sessions, request, 9));
    f.drain();
    assert!(f.sessions.get(42).is_none());
    assert!(!f.server.runtime.as_ref().unwrap().keys.contains_key(&42));
    assert_eq!(f.server.epoch_barrier(), 9);
}

#[test]
fn g2_r3_interleaved_open_completions_keep_live_plus_reserved_within_limit() {
    const LIMIT: usize = 1024;
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    for client in 1..=LIMIT as u64 {
        f.send(
            client,
            &peer,
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
        );
    }
    assert_eq!(f.slots(), LIMIT);
    let first = f.commands.try_recv().unwrap();
    f.complete(first);
    let second = f.commands.try_recv().unwrap();
    f.complete(second);
    assert_eq!(f.sessions.len(), 2);
    f.send(1, &peer, MClientSession::new(SessionOp::REQUEST_OPEN, 0)); // Duplicate attachment reserves no new slot.
    f.send(3, &peer, MClientSession::new(SessionOp::REQUEST_OPEN, 0)); // Duplicate pending OPEN too.
    assert_eq!(f.slots(), LIMIT);
    f.send(2000, &peer, MClientSession::new(SessionOp::REQUEST_OPEN, 0));
    assert!(
        f.slots() <= LIMIT,
        "new OPEN must reserve its slot before the store command"
    );
    assert!(f.server.fatal_error().is_none());
    f.drain();
    assert_eq!(f.sessions.len(), LIMIT);
    assert!(!f.durable_keys.contains_key(&2000));
    f.send(1, &peer, MClientSession::new(SessionOp::REQUEST_CLOSE, 0));
    f.drain();
    f.open(2000, &peer);
    assert_eq!(f.slots(), LIMIT);
}

#[test]
fn g2_r3_open_reservation_survives_stale_owner_but_releases_on_failure_or_cancel() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.send(42, &peer, MClientSession::new(SessionOp::REQUEST_OPEN, 0));
    let command = f.commands.try_recv().unwrap();
    assert_eq!(f.slots(), 1);
    let mut stale_owner = command.owner;
    stale_owner.gid += 1;
    f.server.on_storage_event(
        &mut f.sessions,
        stale_owner,
        command.operation,
        EventKind::Failed(store::StoreError::Io),
    );
    assert_eq!(
        f.slots(),
        1,
        "an unrelated callback cannot release the current operation's slot"
    );
    f.send(42, &peer, MClientSession::new(SessionOp::REQUEST_OPEN, 0));
    assert_eq!(f.slots(), 1);
    assert!(f.commands.try_recv().is_err());
    f.server.on_storage_event(
        &mut f.sessions,
        command.owner,
        command.operation,
        EventKind::Failed(store::StoreError::Io),
    );
    assert!(f.server.fatal_error().is_some());
    assert_eq!(f.slots(), 0);
    assert_eq!(f.server.runtime.as_ref().unwrap().opening_bytes, 0);
    let mut f = Fixture::new();
    f.send(42, &peer, MClientSession::new(SessionOp::REQUEST_OPEN, 0));
    assert_eq!(f.slots(), 1);
    f.server.protocol_error(42);
    assert_eq!(f.slots(), 0);
    assert_eq!(f.server.runtime.as_ref().unwrap().opening_bytes, 0);
}

#[test]
fn g2_r4_busy_cap_does_not_hide_queued_lifecycle_completion() {
    let mut f = Fixture::new();
    let writer = Peer::new(10);
    f.open(42, &writer);
    let ino = f.create(42, &writer);
    let (tx, mut evictions) = mpsc::channel(4);
    f.server.set_evict_channel(tx);
    f.server.request_eviction(&mut f.sessions, 42);
    assert!(
        f.server
            .finish_eviction(&mut f.sessions, evictions.try_recv().unwrap(), 9)
    );
    f.drain();
    let job = f
        .server
        .ready_jobs()
        .into_iter()
        .find(|job| matches!(job.kind, engine::LifecycleKind::RecoverSize { .. }))
        .unwrap();
    let reader = Peer::new(20);
    f.open(43, &reader);
    // The engine returns Busy before cap validation while this lost writer's
    // size recovery is pending. No write cap may be trusted in that interval.
    f.send(
        43,
        &reader,
        MClientCaps {
            op: CapOp::FLUSH,
            ino,
            dirty: proto::types::cap::FILE_WR,
            tid: 99,
            ..Default::default()
        },
    );
    assert!(!f.server.has_work(), "all-Busy input must not spin");
    assert_eq!(f.server.runtime.as_ref().unwrap().control.len(), 1);
    let r = f.server.runtime.as_mut().unwrap();
    let key = r.keys[&43];
    let input = r.control.front().unwrap().input.clone();
    assert!(matches!(input, EngineInput::CapUpdate { key: k, .. } if k == key));
    for _ in 0..36 {
        r.enqueue(input.clone(), 512).unwrap();
    }
    let bytes = r.control_bytes;
    let mut turns = 0;
    while f.server.has_work() {
        f.server.pump(&mut f.sessions, 1);
        turns += 1;
        assert!(turns <= 37, "a complete all-Busy pass must stop scheduling");
    }
    assert_eq!(f.server.runtime.as_ref().unwrap().control.len(), 37);
    assert_eq!(f.server.runtime.as_ref().unwrap().control_bytes, bytes);
    assert!(f.commands.try_recv().is_err());
    f.server.lifecycle_completed(
        job.id,
        engine::LifecycleResult::Recovered {
            size: 1234,
            mtime: engine::Timestamp::new(42, 0),
        },
    );
    for _ in 0..256 {
        if !f.server.has_work() {
            break;
        }
        f.server.pump(&mut f.sessions, 1);
        if f.server.engine.journal().busy() {
            break;
        }
    }
    let completion = f
        .commands
        .try_recv()
        .expect("queued lifecycle completion must start without Tick or new input");
    assert!(
        matches!(&completion.kind, CommandKind::Append { body, .. } if store::record::decode_record(body).unwrap().completed_jobs.contains(&job.id))
    );
    f.complete(completion);
    f.drain();
    assert!(!f.server.has_job(job.id));
    assert!(f.server.runtime.as_ref().unwrap().control.is_empty());
    assert!(!f.server.has_work());
    assert_eq!(f.server.engine.inode(ino).unwrap().size, 1234);
}
