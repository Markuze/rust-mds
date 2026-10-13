//! Loopback tests: a server messenger (MDS type) against a client messenger and against a
//! hand-driven msgr1 client that behaves like the kernel (banner + address at once, cephx
//! authorizer with one challenge round, `SEQ` handshake, signatures, explicit acks).

use std::sync::Arc;
use std::time::Duration;

use auth::service::MonTicketIssuer;
use auth::{AuthContext, AuthName, ClientAuthorizer, CryptoKey, SessionSigner};
use bytes::{Bytes, BytesMut};
use denc::{EntityAddr, EntityName, EntityType, RawMessage, UTime, features};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::proto::{FrameReader, InFrame};
use crate::v1::{V1Reader, V1Writer};
use crate::wire::*;
use crate::{Event, Messenger};

struct Fixture {
    issuer: MonTicketIssuer,
    server: Messenger,
    server_events: mpsc::Receiver<Event>,
}

fn now() -> UTime {
    UTime::now()
}

async fn fixture() -> Fixture {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .try_init();
    let mds_name = AuthName::new(EntityType::MDS, "a");
    let mds_key = CryptoKey::generate_aes(now());
    let issuer = MonTicketIssuer::new(mds_name.clone(), mds_key.clone(), 1);
    let ctx = AuthContext::new(mds_name, mds_key, auth::ENTITY_MON | auth::ENTITY_OSD);
    ctx.set_rotating(issuer.mds_keystore().rotating);
    let (server, server_events) = Messenger::bind(
        EntityName::new(EntityType::MDS, 0),
        "127.0.0.1:0".parse().unwrap(),
        ctx,
    )
    .await
    .unwrap();
    Fixture {
        issuer,
        server,
        server_events,
    }
}

fn client_ctx(issuer: &MonTicketIssuer, id: &str, global_id: u64) -> Arc<AuthContext> {
    let name = AuthName::new(EntityType::CLIENT, id);
    let key = CryptoKey::generate_aes(now());
    let ctx = AuthContext::new(name.clone(), key.clone(), auth::ENTITY_MDS);
    *ctx.client() = issuer.client_for(name, key, global_id).unwrap();
    ctx
}

async fn next_event(rx: &mut mpsc::Receiver<Event>) -> Event {
    timeout(Duration::from_secs(10), rx.recv())
        .await
        .expect("event timeout")
        .expect("channel closed")
}

fn msg(ty: u16, text: &'static [u8]) -> RawMessage {
    RawMessage::new(ty, 1, 1, Bytes::from_static(text))
}

#[tokio::test]
async fn messenger_to_messenger_with_challenge_and_signatures() {
    let mut f = fixture().await;
    let cctx = client_ctx(&f.issuer, "admin", 4100);
    let (client, mut cev) = Messenger::bind(
        EntityName::new(EntityType::CLIENT, 4100),
        "127.0.0.1:0".parse().unwrap(),
        cctx,
    )
    .await
    .unwrap();
    let conn = client.connect(f.server.my_addr(), EntityType::MDS);
    conn.send(msg(22, b"open please"));

    let Event::Connected { .. } = next_event(&mut cev).await else {
        panic!("expected Connected")
    };
    let Event::Accepted { conn: sconn } = next_event(&mut f.server_events).await else {
        panic!("expected Accepted")
    };
    assert_eq!(sconn.peer_global_id(), 4100);
    assert_eq!(sconn.peer_auth_name().unwrap().to_string(), "client.admin");
    assert_eq!(
        sconn.peer_caps().unwrap().cap_string().as_deref(),
        Some("allow")
    );
    assert_ne!(sconn.features() & features::MSG_AUTH, 0);
    let Event::Message { msg: m, .. } = next_event(&mut f.server_events).await else {
        panic!("expected Message")
    };
    assert_eq!(m.msg_type, 22);
    assert_eq!(&m.front[..], b"open please");
    assert_eq!(m.src, Some(EntityName::new(EntityType::CLIENT, 4100)));

    for i in 0..5u8 {
        sconn.send(RawMessage::new(23, 1, 1, Bytes::from(vec![i; 100])));
    }
    for i in 0..5u8 {
        let Event::Message { msg: m, .. } = next_event(&mut cev).await else {
            panic!("expected Message")
        };
        assert_eq!(m.front[0], i);
        assert_eq!(m.src, Some(EntityName::new(EntityType::MDS, 0)));
    }
    // Acks trim the server's unacknowledged list.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (out_seq, _in_seq, unacked, _) = sconn.seq_state();
    assert_eq!(out_seq, 5);
    assert_eq!(unacked, 0);
}

/// A minimal kernel-like msgr1 client.
struct RawClient {
    stream: TcpStream,
    authorizer: ClientAuthorizer,
    features: u64,
    global_seq: u32,
}

struct Opened {
    reader: V1Reader,
    writer: tokio::net::tcp::OwnedWriteHalf,
    signer: Option<SessionSigner>,
    tag: u8,
    server_in_seq: u64,
}

impl RawClient {
    async fn connect(server: EntityAddr, ctx: &AuthContext, nonce: u32) -> RawClient {
        let mut stream = TcpStream::connect(server.addr.unwrap()).await.unwrap();
        let mut b = [0u8; 9 + 2 * ADDR_LEN];
        // The kernel sends banner and address together, before reading.
        let my_addr = EntityAddr::new(denc::AddrType::LEGACY, "0.0.0.0:0".parse().unwrap(), nonce);
        let mut out = BANNER.to_vec();
        let mut a = BytesMut::new();
        my_addr.encode_legacy(&mut a);
        out.extend_from_slice(&a);
        stream.write_all(&out).await.unwrap();
        stream.read_exact(&mut b).await.unwrap();
        assert_eq!(&b[..9], BANNER);
        let mut sa = Bytes::copy_from_slice(&b[9..9 + ADDR_LEN]);
        let saddr = EntityAddr::decode_legacy(&mut sa).unwrap();
        assert_eq!(saddr.addr, server.addr);
        assert_eq!(saddr.nonce, server.nonce);
        let authorizer = ctx.build_authorizer(EntityType::MDS).unwrap();
        RawClient {
            stream,
            authorizer,
            features: features::ALL,
            global_seq: 1,
        }
    }

    async fn send_connect(&mut self, connect_seq: u32, auth: bool) {
        let bl = if auth {
            self.authorizer.bl.clone()
        } else {
            Bytes::new()
        };
        let c = ConnectMsg {
            features: self.features,
            host_type: 8,
            global_seq: self.global_seq,
            connect_seq,
            protocol_version: CEPH_MDSC_PROTOCOL,
            authorizer_protocol: if auth { 2 } else { 0 },
            authorizer_len: bl.len() as u32,
            flags: 0,
        };
        let mut out = c.to_bytes().to_vec();
        out.extend_from_slice(&bl);
        self.stream.write_all(&out).await.unwrap();
    }

    async fn read_reply(&mut self) -> (ConnectReply, Vec<u8>) {
        let mut rb = [0u8; ConnectReply::LEN];
        self.stream.read_exact(&mut rb).await.unwrap();
        let r = ConnectReply::from_bytes(&rb);
        let mut a = vec![0u8; r.authorizer_len as usize];
        self.stream.read_exact(&mut a).await.unwrap();
        (r, a)
    }

    /// Connect with one challenge round; returns the final reply.
    async fn handshake(mut self, connect_seq: u32, my_in_seq: u64) -> Opened {
        self.send_connect(connect_seq, true).await;
        let (r, a) = self.read_reply().await;
        assert_eq!(r.tag, tag::CHALLENGE_AUTHORIZER);
        self.authorizer.add_challenge(&a).unwrap();
        self.send_connect(connect_seq, true).await;
        let (r, a) = self.read_reply().await;
        self.finish(r, a, my_in_seq).await
    }

    async fn finish(mut self, r: ConnectReply, a: Vec<u8>, my_in_seq: u64) -> Opened {
        assert!(
            r.tag == tag::SEQ || r.tag == tag::READY,
            "unexpected tag {}",
            tag::name(r.tag)
        );
        assert_ne!(
            r.features & features::MSG_AUTH,
            0,
            "kernel requires MSG_AUTH"
        );
        self.authorizer.verify_reply(&a).unwrap();
        let mut server_in_seq = 0;
        if r.tag == tag::SEQ {
            server_in_seq = self.stream.read_u64_le().await.unwrap();
            self.stream
                .write_all(&my_in_seq.to_le_bytes())
                .await
                .unwrap();
        }
        let f = r.features & self.features;
        let signer = SessionSigner::new(self.authorizer.session_key.clone(), f);
        let (rd, wr) = self.stream.into_split();
        Opened {
            reader: V1Reader::new(rd, true, signer.clone()),
            writer: wr,
            signer,
            tag: r.tag,
            server_in_seq,
        }
    }
}

impl Opened {
    async fn send_msg(&mut self, seq: u64, m: &RawMessage) {
        let mut buf = Vec::new();
        V1Writer::encode_message(
            &mut buf,
            seq,
            m,
            EntityName::new(EntityType::CLIENT, 4200),
            true,
            self.signer.as_ref(),
        )
        .unwrap();
        self.writer.write_all(&buf).await.unwrap();
    }

    async fn send_raw(&mut self, b: &[u8]) {
        self.writer.write_all(b).await.unwrap();
    }

    async fn read(&mut self) -> InFrame {
        timeout(Duration::from_secs(10), self.reader.read_frame())
            .await
            .expect("frame timeout")
            .unwrap()
    }

    /// Read frames until a message arrives (acks are skipped).
    async fn read_msg(&mut self) -> (u64, RawMessage) {
        loop {
            match self.read().await {
                InFrame::Message { seq, msg } => return (seq, msg),
                InFrame::Ack(_) => continue,
                f => panic!("unexpected frame {f:?}"),
            }
        }
    }
}

#[tokio::test]
async fn kernel_like_client_session_lifecycle() {
    let mut f = fixture().await;
    let cctx = client_ctx(&f.issuer, "k", 4200);
    let server = f.server.my_addr();

    // 1. New session: challenge, then SEQ with in_seq 0.
    let rc = RawClient::connect(server, &cctx, 77).await;
    let mut o = rc.handshake(0, 0).await;
    assert_eq!(o.tag, tag::SEQ);
    assert_eq!(o.server_in_seq, 0);
    let Event::Accepted { conn: sconn } = next_event(&mut f.server_events).await else {
        panic!()
    };
    assert_eq!(sconn.peer_global_id(), 4200);

    // A signed message from the client is delivered and acknowledged.
    o.send_msg(1, &msg(22, b"session open")).await;
    let Event::Message { msg: m, .. } = next_event(&mut f.server_events).await else {
        panic!()
    };
    assert_eq!(&m.front[..], b"session open");
    match o.read().await {
        InFrame::Ack(1) => {}
        fr => panic!("expected ack 1, got {fr:?}"),
    }
    // KEEPALIVE2 is echoed with the same timestamp.
    let mut ka = vec![tag::KEEPALIVE2];
    ka.extend_from_slice(&1234u32.to_le_bytes());
    ka.extend_from_slice(&5678u32.to_le_bytes());
    o.send_raw(&ka).await;
    match o.read().await {
        InFrame::Keepalive2Ack(t) => assert_eq!(
            t,
            UTime {
                sec: 1234,
                nsec: 5678
            }
        ),
        fr => panic!("expected keepalive2 ack, got {fr:?}"),
    }
    // Server messages carry sequence 1, 2 and valid signatures (checked by V1Reader).
    sconn.send(msg(22, b"one"));
    sconn.send(msg(22, b"two"));
    assert_eq!(o.read_msg().await.0, 1);
    assert_eq!(o.read_msg().await.0, 2);
    // Ack only the first, then drop the socket: the server keeps message 2.
    o.send_raw(&[&[tag::ACK][..], &1u64.to_le_bytes()].concat())
        .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(o);
    tokio::time::sleep(Duration::from_millis(200)).await;
    sconn.send(msg(22, b"three")); // queued while in standby

    // 2. Reconnect. A stale connect_seq (1, equal to the session's) gets RETRY_SESSION with 2;
    //    retrying with 2 gets SEQ with the server's in_seq (1); message 2 is replayed with its
    //    original number, then 3.
    let mut rc = RawClient::connect(server, &cctx, 77).await;
    rc.send_connect(1, true).await;
    let (r, a) = rc.read_reply().await;
    assert_eq!(r.tag, tag::CHALLENGE_AUTHORIZER);
    rc.authorizer.add_challenge(&a).unwrap();
    rc.send_connect(1, true).await;
    let (r, _) = rc.read_reply().await;
    assert_eq!((r.tag, r.connect_seq), (tag::RETRY_SESSION, 2));
    rc.send_connect(2, true).await;
    let (r, a) = rc.read_reply().await;
    let mut o = rc.finish(r, a, 1).await;
    assert_eq!(o.tag, tag::SEQ);
    assert_eq!(o.server_in_seq, 1);
    let (s2, m2) = o.read_msg().await;
    assert_eq!((s2, &m2.front[..]), (2, &b"two"[..]));
    let (s3, m3) = o.read_msg().await;
    assert_eq!((s3, &m3.front[..]), (3, &b"three"[..]));
    let Event::Accepted { conn: again } = next_event(&mut f.server_events).await else {
        panic!()
    };
    assert_eq!(again, sconn, "same session after reconnect");
    // The client continues its numbering.
    o.send_msg(2, &msg(24, b"after reconnect")).await;
    let Event::Message { msg: m, conn } = next_event(&mut f.server_events).await else {
        panic!()
    };
    assert_eq!(conn, sconn);
    assert_eq!(&m.front[..], b"after reconnect");
    drop(o);

    // 3. Peer reset: connect_seq 0 against the existing session -> READY, old session reset.
    let rc = RawClient::connect(server, &cctx, 77).await;
    let o = rc.handshake(0, 0).await;
    assert_eq!(o.tag, tag::READY);
    let mut saw_reset = false;
    let mut new_conn = None;
    for _ in 0..2 {
        match next_event(&mut f.server_events).await {
            Event::Reset { conn } => {
                assert_eq!(conn, sconn);
                saw_reset = true;
            }
            Event::Accepted { conn } => new_conn = Some(conn),
            e => panic!("unexpected {e:?}"),
        }
    }
    assert!(saw_reset);
    assert_ne!(new_conn.unwrap(), sconn);
    assert!(!sconn.send(msg(1, b"x")), "old session is dead");
    drop(o);

    // 4. Unknown session (another nonce) with connect_seq > 0 -> RESETSESSION.
    let mut rc = RawClient::connect(server, &cctx, 78).await;
    rc.send_connect(5, true).await;
    let (r, a) = rc.read_reply().await;
    assert_eq!(r.tag, tag::CHALLENGE_AUTHORIZER);
    rc.authorizer.add_challenge(&a).unwrap();
    rc.send_connect(5, true).await;
    let (r, _) = rc.read_reply().await;
    assert_eq!(r.tag, tag::RESETSESSION);
}

#[tokio::test]
async fn connect_errors_are_answered_with_tags() {
    let f = fixture().await;
    let cctx = client_ctx(&f.issuer, "bad", 4300);
    let server = f.server.my_addr();

    // No authorizer -> BADAUTHORIZER.
    let mut rc = RawClient::connect(server, &cctx, 90).await;
    rc.send_connect(0, false).await;
    assert_eq!(rc.read_reply().await.0.tag, tag::BADAUTHORIZER);
    // Wrong protocol version -> BADPROTOVER.
    let c = ConnectMsg {
        features: features::ALL,
        host_type: 8,
        global_seq: 1,
        connect_seq: 0,
        protocol_version: 99,
        ..Default::default()
    };
    rc.stream.write_all(&c.to_bytes()).await.unwrap();
    assert_eq!(rc.read_reply().await.0.tag, tag::BADPROTOVER);
    // Missing CEPHX_V2 with a cephx authorizer -> FEATURES.
    rc.features = features::ALL & !features::CEPHX_V2;
    rc.send_connect(0, true).await;
    assert_eq!(rc.read_reply().await.0.tag, tag::FEATURES);
    // A tampered authorizer -> BADAUTHORIZER (after the challenge round is skipped or not).
    rc.features = features::ALL;
    let mut bl = rc.authorizer.bl.to_vec();
    let n = bl.len();
    bl[n - 1] ^= 0xff;
    let c = ConnectMsg {
        features: features::ALL,
        host_type: 8,
        global_seq: 1,
        connect_seq: 0,
        protocol_version: CEPH_MDSC_PROTOCOL,
        authorizer_protocol: 2,
        authorizer_len: n as u32,
        flags: 0,
    };
    let mut out = c.to_bytes().to_vec();
    out.extend_from_slice(&bl);
    rc.stream.write_all(&out).await.unwrap();
    assert_eq!(rc.read_reply().await.0.tag, tag::BADAUTHORIZER);
}

#[tokio::test]
async fn caps_validator_rejects() {
    let f = fixture().await;
    f.server.set_caps_validator(|v| v.name.id != "evil");
    let cctx = client_ctx(&f.issuer, "evil", 4400);
    let rc = RawClient::connect(f.server.my_addr(), &cctx, 91).await;
    let mut rc = rc;
    rc.send_connect(0, true).await;
    let (r, a) = rc.read_reply().await;
    assert_eq!(r.tag, tag::CHALLENGE_AUTHORIZER);
    rc.authorizer.add_challenge(&a).unwrap();
    rc.send_connect(0, true).await;
    assert_eq!(rc.read_reply().await.0.tag, tag::BADAUTHORIZER);
}

#[tokio::test]
async fn lossless_client_reconnects_and_replays() {
    let mut f = fixture().await;
    let cctx = client_ctx(&f.issuer, "replay", 4500);
    let (client, mut cev) = Messenger::bind(
        EntityName::new(EntityType::CLIENT, 4500),
        "127.0.0.1:0".parse().unwrap(),
        cctx,
    )
    .await
    .unwrap();
    let conn = client.connect(f.server.my_addr(), EntityType::MDS);
    let Event::Connected { .. } = next_event(&mut cev).await else {
        panic!()
    };
    let Event::Accepted { conn: sconn } = next_event(&mut f.server_events).await else {
        panic!()
    };
    for i in 0..3u8 {
        conn.send(RawMessage::new(30, 1, 1, Bytes::from(vec![i])));
    }
    for i in 0..3u8 {
        let Event::Message { msg: m, .. } = next_event(&mut f.server_events).await else {
            panic!()
        };
        assert_eq!(m.front[0], i);
    }
    // Break the server's socket: the server session waits in standby, the client reconnects
    // (connect_seq + 1) and both sides replay what the other did not acknowledge.
    sconn.inject_socket_failure();
    for i in 3..6u8 {
        conn.send(RawMessage::new(30, 1, 1, Bytes::from(vec![i])));
        sconn.send(RawMessage::new(31, 1, 1, Bytes::from(vec![i])));
    }
    let mut got_server = Vec::new();
    let mut got_client = Vec::new();
    let mut reaccepted = false;
    while got_server.len() < 3 || got_client.len() < 3 {
        tokio::select! {
            e = next_event(&mut f.server_events) => match e {
                Event::Message { msg, conn: c } => { assert_eq!(c, sconn); got_server.push(msg.front[0]); }
                Event::Accepted { conn: c } => { assert_eq!(c, sconn, "same session"); reaccepted = true; }
                e => panic!("unexpected server event {e:?}"),
            },
            e = next_event(&mut cev) => match e {
                Event::Message { msg, .. } => got_client.push(msg.front[0]),
                Event::Connected { .. } => {}
                e => panic!("unexpected client event {e:?}"),
            },
        }
    }
    assert!(reaccepted);
    assert_eq!(got_server, vec![3, 4, 5], "exactly once, in order");
    assert_eq!(got_client, vec![3, 4, 5]);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        sconn.seq_state().1,
        6,
        "server in_seq continues across the reconnect"
    );
}
