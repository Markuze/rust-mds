//! Test-only production worker/dispatch harness. It never contacts a Ceph cluster.

use crate::{
    fencing::Membership,
    persistence::*,
    server::Server,
    session::{ClientLink, SessionMap},
    test_backend::{Fence, MemoryBackend},
};
use denc::{Bytes, EntityAddr, EntityAddrVec, Message, RawMessage};
use engine::EngineConfig;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, watch};

#[derive(Clone)]
pub struct Peer {
    pub connection: u64,
    pub addr: EntityAddr,
    pub auth: String,
    pub output: Arc<Mutex<Vec<RawMessage>>>,
}
impl Peer {
    pub fn new(nonce: u32) -> Self {
        Self {
            connection: u64::from(nonce),
            addr: EntityAddr::new(
                denc::AddrType::LEGACY,
                "127.0.0.1:4000".parse().unwrap(),
                nonce,
            ),
            auth: "client.test".into(),
            output: Default::default(),
        }
    }
    pub fn clear(&self) {
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
        Some((self.auth.clone(), Bytes::new()))
    }
}
pub fn send<M: Message>(
    server: &mut Server,
    sessions: &mut SessionMap,
    client: u64,
    peer: &Peer,
    msg: M,
) {
    crate::daemon::dispatch_client_raw(
        server,
        sessions,
        client,
        Box::new(peer.clone()),
        vec![],
        msg.to_raw(peer.features()),
    );
}

pub struct Worker {
    pub tx: mpsc::Sender<Command>,
    pub rx: mpsc::Receiver<Event>,
    pub authority: watch::Sender<Authority>,
    pub config: EngineConfig,
    pub attempts: Attempts,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Worker {
    pub async fn load(
        backend: MemoryBackend,
        incarnation: u64,
        creating: bool,
        config: EngineConfig,
    ) -> (Loaded, Self) {
        // Logical fixture assignments leave one real successor for adoption.
        // No Loaded fixture is made from the seed process.
        if creating {
            let mut seed = Self::launch(
                backend.clone(),
                incarnation * 2 - 1,
                true,
                true,
                config.clone(),
            );
            let event = seed.event().await;
            assert_eq!(event.owner, None);
            assert_eq!(event.operation, 1);
            assert!(matches!(
                event.kind,
                EventKind::Failed(store::StoreError::UpgradeRequired)
            ));
            assert!(seed.rx.recv().await.is_none());
            let _ = (&mut seed.task).await;
        }
        let mut worker = Self::launch(backend, incarnation * 2, false, creating, config);
        let event = worker.event().await;
        let EventKind::Loaded(loaded) = event.kind else {
            panic!("real successor worker did not load");
        };
        assert_eq!(event.membership.incarnation as u64, incarnation * 2);
        (*loaded, worker)
    }
    pub fn launch(
        backend: MemoryBackend,
        incarnation: u64,
        creating: bool,
        upgrade_retry_targets: bool,
        config: EngineConfig,
    ) -> Self {
        let membership = Membership {
            rank: 0,
            incarnation: incarnation as i32,
            gid: incarnation,
        };
        let (authority, rx) = watch::channel(Authority {
            membership: Some(membership),
            state: if creating {
                monc::MdsState::CREATING
            } else {
                monc::MdsState::REPLAY
            },
            healthy: true,
        });
        let attempts = Attempts::default();
        let start = Start {
            identity: store::StoreIdentity {
                cluster_fsid: [7; 16],
                filesystem_id: 3,
                metadata_pool: 6,
            },
            writer: store::WriterIdentity {
                rank_owner: rank_owner(membership).unwrap(),
                addresses: EntityAddrVec(vec![EntityAddr::new(
                    denc::AddrType::LEGACY,
                    "127.0.0.1:7402".parse().unwrap(),
                    incarnation as u32,
                )]),
            },
            config: config.clone(),
            creating,
            upgrade_retry_targets,
            attempts: attempts.clone(),
            failure_epoch: 0,
            limits: store::Limits::default(),
            operation: 1,
        };
        let (tx, commands) = mpsc::channel(64);
        let (events, rx_events) = mpsc::channel(64);
        let task = tokio::spawn(worker(
            backend.clone(),
            start,
            Gate {
                rx,
                expected: membership,
            },
            Fence::new(backend),
            commands,
            events,
        ));
        Self {
            tx,
            rx: rx_events,
            authority,
            config,
            attempts,
            task,
        }
    }
    pub fn server(&self, loaded: Loaded) -> Server {
        Server::from_loaded(
            "a".into(),
            self.config.default_pool(),
            1,
            loaded,
            self.config.clone(),
            &store::Limits::default(),
            self.tx.clone(),
            1,
            10000,
            Duration::from_secs(3600),
        )
        .unwrap()
    }
    pub async fn event(&mut self) -> Event {
        tokio::time::timeout(Duration::from_secs(3), self.rx.recv())
            .await
            .unwrap()
            .unwrap()
    }
    pub fn apply(server: &mut Server, sessions: &mut SessionMap, event: Event) {
        server.on_storage_event(sessions, event.owner.unwrap(), event.operation, event.kind);
        assert!(server.fatal_error().is_none(), "{:?}", server.fatal_error());
    }
    pub async fn settle(&mut self, server: &mut Server, sessions: &mut SessionMap) {
        for _ in 0..4096 {
            server.pump(sessions, 32);
            if server.pending_storage() > 0 {
                Self::apply(server, sessions, self.event().await);
            } else if !server.has_work() {
                return;
            }
        }
        panic!("real worker did not settle");
    }
    pub async fn ready(&mut self, server: &mut Server, sessions: &mut SessionMap) {
        self.settle(server, sessions).await;
        assert!(!server.try_finish_recovery(sessions));
        self.settle(server, sessions).await;
        assert!(server.try_finish_recovery(sessions));
        server.pause_data_work(false);
    }
    pub async fn stop(&mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
    }
}

pub async fn create(
    worker: &mut Worker,
    server: &mut Server,
    sessions: &mut SessionMap,
    peer: &Peer,
) -> proto::InodeStat {
    send(
        server,
        sessions,
        42,
        peer,
        proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0),
    );
    worker.settle(server, sessions).await;
    let mut request = proto::MClientRequest::new(1, proto::MdsOp::CREATE);
    request.path = proto::FilePath::new(1, "file");
    request.head.flags = proto::request::request_flags::WANT_DENTRY;
    request.head.args = proto::request::OpenArgs {
        flags: proto::open_flags::CREAT | proto::open_flags::RDWR,
        mode: 0o644,
        pool: worker.config.default_pool().try_into().unwrap(),
        ..Default::default()
    }
    .to_args();
    send(server, sessions, 42, peer, request);
    worker.settle(server, sessions).await;
    let reply = peer
        .output
        .lock()
        .unwrap()
        .iter()
        .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_REPLY)
        .map(|raw| proto::MClientReply::from_raw(raw.clone()).unwrap())
        .find(|r| r.tid == 1)
        .unwrap();
    assert_eq!(reply.result, 0);
    reply.trace.target.unwrap()
}
