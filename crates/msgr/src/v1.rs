//! msgr1 ("legacy") protocol: banner exchange, `ceph_msg_connect` negotiation and message
//! framing. Follows `src/msg/async/ProtocolV1.cc`.
//!
//! **Accepting** (`send_server_banner` .. `server_ready`): write `"ceph v027"` + our legacy
//! address + the peer's socket address; read the peer's banner and address (a blank IP is
//! replaced by the socket's IP); then loop on `ceph_msg_connect` + authorizer:
//! protocol check (`BADPROTOVER`), required features (`FEATURES`; cephx requires `CEPHX_V2`),
//! authorizer verification (`BADAUTHORIZER`, or `CHALLENGE_AUTHORIZER` for the one challenge
//! round of CEPHX_V2 peers), then the session rules against an existing session of the same
//! peer address (`RETRY_GLOBAL`, `RETRY_SESSION`, `RESETSESSION`, replace, or a new session).
//! The final reply is `SEQ` when the peer has `RECONNECT_SEQ` (followed by our `le64 in_seq`,
//! answered with the peer's `le64` acknowledged sequence) and `READY` otherwise.
//! Differences from C++: a reconnect to an existing session is answered on the same socket
//! (C++ moves the socket into the existing connection object and asks the peer to retry with
//! `RETRY_GLOBAL` first); a peer reset under `resetcheck` creates a new session object instead
//! of resetting the old one. `WAIT` is only sent to non-server sessions (never to clients).
//!
//! **Connecting** (`send_client_banner` .. `client_ready`): banner + our address, check the
//! server's banner address, `ceph_msg_connect` (no authorizer towards monitors), then follow
//! the reply tags.
//!
//! **Open state** (`handle_message`, `write_message`): tags `MSG` (header, front, middle, data,
//! footer), `ACK`, `KEEPALIVE`, `KEEPALIVE2`, `KEEPALIVE2_ACK`, `CLOSE`. Header crc and front
//! and middle crcs are always checked; the data crc unless the footer has `NOCRC`; the cephx
//! signature when `MSG_AUTH` was negotiated and there is a session key.

use std::net::SocketAddr;
use std::sync::Arc;

use auth::{AUTH_CEPHX, AuthRequestOutcome, AuthorizerState, ClientAuthorizer, SessionSigner, SigFields};
use bytes::{Bytes, BytesMut};
use denc::{AddrType, EntityAddr, EntityName, EntityType, RawMessage, UTime, features};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tracing::{debug, info, warn};

use crate::conn::{ConnShared, MAX_SEGMENT, PeerAuth, Phase, proto_err};
use crate::messenger::MsgrShared;
use crate::proto::{FrameReader, FrameWriter, InFrame, OutFrame};
use crate::wire::*;
use crate::{Event, MsgrError, Policy};

async fn read_array<const N: usize, R: AsyncRead + Unpin>(r: &mut R) -> Result<[u8; N], MsgrError> {
    let mut b = [0u8; N];
    r.read_exact(&mut b).await?;
    Ok(b)
}

async fn read_vec<R: AsyncRead + Unpin>(r: &mut R, n: usize) -> Result<Vec<u8>, MsgrError> {
    let mut b = vec![0u8; n];
    r.read_exact(&mut b).await?;
    Ok(b)
}

fn encode_addr(a: &EntityAddr) -> Vec<u8> {
    let mut b = BytesMut::new();
    a.encode_legacy(&mut b);
    b.to_vec()
}

fn decode_addr(b: &[u8]) -> Result<EntityAddr, MsgrError> {
    let mut bytes = Bytes::copy_from_slice(b);
    EntityAddr::decode_legacy(&mut bytes).map_err(|e| proto_err(format!("bad banner address: {e}")))
}

fn is_blank_ip(a: &EntityAddr) -> bool {
    a.addr.is_none_or(|s| s.ip().is_unspecified())
}

/// Registry key of a peer (its legacy address with nonce).
pub(crate) fn peer_key(a: &EntityAddr) -> (Option<SocketAddr>, u32) {
    (a.addr, a.nonce)
}

// ---------------------------------------------------------------------------------------
// Accepting side.

/// Outcome of the session rules for one connect message.
enum Decision {
    Reply { tag: u8, global_seq: u32, connect_seq: u32 },
    New { reset_old: Option<Arc<ConnShared>>, peer_reset: bool },
    Replace { conn: Arc<ConnShared>, peer_reset: bool },
}

/// The existing-session rules of `ProtocolV1::handle_connect_message_2`.
fn decide(msgr: &MsgrShared, peer_addr: &EntityAddr, c: &ConnectMsg, policy: &Policy) -> Decision {
    if policy.server && policy.lossy {
        // Incoming lossy client: not registered, always a new session.
        return Decision::New { reset_old: None, peer_reset: false };
    }
    let existing = msgr.lookup(peer_addr).filter(|e| e.st.lock().unwrap().phase != Phase::Closed);
    let Some(e) = existing else {
        if c.connect_seq > 0 {
            // We reset, and they are opening a new session.
            return Decision::Reply { tag: tag::RESETSESSION, global_seq: 0, connect_seq: 0 };
        }
        return Decision::New { reset_old: None, peer_reset: false };
    };
    let st = e.st.lock().unwrap();
    if !e.outgoing && st.phase == Phase::Connecting {
        // Another accept is attaching a socket to this session right now (C++ `replacing`).
        return Decision::Reply { tag: tag::RETRY_GLOBAL, global_seq: st.peer_global_seq, connect_seq: 0 };
    }
    if c.global_seq < st.peer_global_seq {
        return Decision::Reply { tag: tag::RETRY_GLOBAL, global_seq: st.peer_global_seq, connect_seq: 0 };
    }
    if st.lossy {
        drop(st);
        return Decision::New { reset_old: Some(e), peer_reset: false };
    }
    if c.connect_seq == 0 && st.connect_seq > 0 {
        // The peer reset its side and connects afresh.
        drop(st);
        if policy.resetcheck {
            return Decision::New { reset_old: Some(e), peer_reset: true };
        }
        return Decision::Replace { conn: e, peer_reset: true };
    }
    if c.connect_seq < st.connect_seq {
        return Decision::Reply { tag: tag::RETRY_SESSION, global_seq: 0, connect_seq: st.connect_seq + 1 };
    }
    if c.connect_seq == st.connect_seq {
        if matches!(st.phase, Phase::Open | Phase::Standby) {
            if policy.resetcheck && st.connect_seq == 0 {
                drop(st);
                return Decision::Replace { conn: e, peer_reset: false };
            }
            return Decision::Reply { tag: tag::RETRY_SESSION, global_seq: 0, connect_seq: st.connect_seq + 1 };
        }
        // Connection race with our own outgoing attempt.
        let incoming_wins = e.policy.server || legacy_lt(peer_addr, &msgr.my_addr);
        drop(st);
        if incoming_wins {
            return Decision::Replace { conn: e, peer_reset: false };
        }
        e.send_keepalive();
        return Decision::Reply { tag: tag::WAIT, global_seq: 0, connect_seq: 0 };
    }
    // connect_seq > existing.
    if policy.resetcheck && st.connect_seq == 0 {
        return Decision::Reply { tag: tag::RESETSESSION, global_seq: 0, connect_seq: 0 };
    }
    drop(st);
    Decision::Replace { conn: e, peer_reset: false }
}

/// `entity_addr_t` ordering (memcmp of the C++ struct) approximated by (ip, port, nonce).
fn legacy_lt(a: &EntityAddr, b: &EntityAddr) -> bool {
    (a.addr, a.nonce) < (b.addr, b.nonce)
}

async fn send_reply(w: &mut TcpStream, mut reply: ConnectReply, auth_reply: &[u8]) -> Result<(), MsgrError> {
    reply.authorizer_len = auth_reply.len() as u32;
    let mut out = reply.to_bytes().to_vec();
    out.extend_from_slice(auth_reply);
    w.write_all(&out).await?;
    Ok(())
}

/// Accept one incoming msgr1 connection.
pub(crate) async fn accept(msgr: Arc<MsgrShared>, mut stream: TcpStream, sock_peer: SocketAddr) -> Result<(), MsgrError> {
    let _ = stream.set_nodelay(true);
    let my_type = msgr.my_name().ty;

    // Banner, our address, and the peer's address as we see it.
    let mut out = BANNER.to_vec();
    out.extend_from_slice(&encode_addr(&msgr.my_addr));
    out.extend_from_slice(&encode_addr(&EntityAddr::new(AddrType::LEGACY, sock_peer, 0)));
    stream.write_all(&out).await?;

    let b: [u8; 9 + ADDR_LEN] = read_array(&mut stream).await?;
    if &b[..9] != BANNER {
        return Err(proto_err(format!("bad banner {:?}", String::from_utf8_lossy(&b[..9]))));
    }
    let mut peer_addr = decode_addr(&b[9..])?;
    if is_blank_ip(&peer_addr) {
        let port = peer_addr.addr.map(|a| a.port()).unwrap_or(0);
        let mut s = sock_peer;
        s.set_port(port);
        peer_addr.addr = Some(s);
    }
    peer_addr.ty = AddrType::LEGACY;
    debug!(%peer_addr, %sock_peer, "accept: banner exchanged");

    let mut auth_state = AuthorizerState::default();
    loop {
        let cb: [u8; ConnectMsg::LEN] = read_array(&mut stream).await?;
        let c = ConnectMsg::from_bytes(&cb);
        if c.authorizer_len > 64 * 1024 {
            return Err(proto_err(format!("authorizer too long: {}", c.authorizer_len)));
        }
        let authorizer = read_vec(&mut stream, c.authorizer_len as usize).await?;
        let peer_type = EntityType(c.host_type as u8);
        let policy = msgr.cfg.policy_for(peer_type);
        debug!(%peer_addr, peer = peer_type.name(), gseq = c.global_seq, cseq = c.connect_seq, features = format_args!("{:#x}", c.features), "accept: connect");

        let mut reply = ConnectReply { protocol_version: proto_version(my_type, peer_type, false), ..Default::default() };
        let mut required = policy.features_required;
        // send_connect_message_reply: features = (peer & supported) | required.
        let nonfinal = |tag: u8, required: u64, mut r: ConnectReply| {
            r.tag = tag;
            r.features = (c.features & msgr.cfg.features) | required;
            r
        };
        if c.protocol_version != reply.protocol_version {
            info!(%peer_addr, theirs = c.protocol_version, ours = reply.protocol_version, "accept: protocol version mismatch");
            send_reply(&mut stream, nonfinal(tag::BADPROTOVER, required, reply), &[]).await?;
            continue;
        }
        if c.authorizer_protocol == AUTH_CEPHX {
            // cephx_{cluster,service}_require_version defaults to 2; signatures are not
            // required by default (cephx_*_require_signatures = false).
            required |= features::MASK_CEPHX_V2;
            if msgr.cfg.require_signatures {
                required |= features::MSG_AUTH;
            }
        }
        if required & !c.features != 0 {
            info!(%peer_addr, missing = format_args!("{:#x}", required & !c.features), "accept: peer lacks required features");
            send_reply(&mut stream, nonfinal(tag::FEATURES, required, reply), &[]).await?;
            continue;
        }
        auth_state.skip_challenge = !features::has(c.features, features::MASK_CEPHX_V2);
        let more = auth_state.challenge.is_some();
        // Monitors accept msgr1 connections without an authorizer (they authenticate with
        // MAuth); everyone else requires one.
        let verified = if authorizer.is_empty() && !msgr.cfg.require_authorizer {
            None
        } else {
            match msgr.auth.handle_auth_request(c.authorizer_protocol, &authorizer, more, &mut auth_state) {
                AuthRequestOutcome::Accept(v) => Some(v),
                AuthRequestOutcome::Challenge(ch) => {
                    debug!(%peer_addr, "accept: challenging authorizer");
                    send_reply(&mut stream, nonfinal(tag::CHALLENGE_AUTHORIZER, required, reply), &ch).await?;
                    continue;
                }
                AuthRequestOutcome::Reject(why) => {
                    warn!(%peer_addr, peer = peer_type.name(), "accept: bad authorizer: {why}");
                    send_reply(&mut stream, nonfinal(tag::BADAUTHORIZER, required, reply), &[]).await?;
                    continue;
                }
            }
        };
        if let Some(v) = &verified
            && !msgr.validate_caps(v)
        {
            warn!(%peer_addr, name = %v.name, "accept: caps refused");
            send_reply(&mut stream, nonfinal(tag::BADAUTHORIZER, required, reply), &[]).await?;
            continue;
        }
        let auth_reply: Bytes = verified.as_ref().map(|v| v.reply.clone()).unwrap_or_default();

        let decision = decide(&msgr, &peer_addr, &c, &policy);
        let (conn, peer_reset) = match decision {
            Decision::Reply { tag: t, global_seq, connect_seq } => {
                info!(%peer_addr, tag = tag::name(t), "accept: session reply");
                reply.global_seq = global_seq;
                reply.connect_seq = connect_seq;
                send_reply(&mut stream, nonfinal(t, required, reply), &auth_reply).await?;
                continue;
            }
            Decision::New { reset_old, peer_reset } => {
                if let Some(old) = reset_old {
                    info!(%peer_addr, old = old.id, "accept: peer reset its session; resetting ours");
                    old.reset("peer reset");
                }
                let conn = msgr.new_conn(policy, false, peer_addr, peer_type, msgr.events.clone());
                if !policy.lossy || !policy.server {
                    msgr.register(&conn);
                }
                (conn, peer_reset)
            }
            Decision::Replace { conn, peer_reset } => {
                info!(%peer_addr, conn = conn.id, "accept: reconnect to existing session");
                (conn, peer_reset)
            }
        };

        // open(): the final reply.
        let use_seq = c.features & features::RECONNECT_SEQ != 0 && !peer_reset;
        let conn_features = msgr.cfg.features & c.features;
        let (sgen, in_seq) = {
            let mut st = conn.st.lock().unwrap();
            let sgen = ConnShared::bump_gen(&mut st);
            st.phase = Phase::Connecting;
            st.connect_seq = c.connect_seq + 1;
            st.peer_global_seq = c.global_seq;
            st.peer_addr = peer_addr;
            st.peer_type = peer_type;
            st.features = conn_features;
            st.signer = verified.as_ref().and_then(|v| SessionSigner::new(v.session_key.clone(), conn_features));
            st.peer_auth = verified.as_ref().map(|v| PeerAuth { name: v.name.clone(), global_id: v.global_id, caps: v.caps.clone() });
            st.resend_from = 0;
            st.keepalive_acks.clear();
            if !use_seq {
                st.in_seq = 0;
                st.acked_in_seq = 0;
                if st.sent.is_empty() {
                    st.out_seq = 0;
                }
            }
            // The SEQ reply carries in_seq, which acknowledges everything up to it.
            st.acked_in_seq = st.in_seq;
            (sgen, st.in_seq)
        };
        reply.tag = if use_seq { tag::SEQ } else { tag::READY };
        reply.features = msgr.cfg.features;
        reply.global_seq = msgr.next_global_seq(0);
        reply.connect_seq = c.connect_seq + 1;
        reply.flags = if policy.lossy { CONNECT_LOSSY } else { 0 };
        reply.authorizer_len = auth_reply.len() as u32;
        let mut out = reply.to_bytes().to_vec();
        out.extend_from_slice(&auth_reply);
        if use_seq {
            out.extend_from_slice(&in_seq.to_le_bytes());
        }
        if let Err(e) = stream.write_all(&out).await {
            conn.fault(sgen, "write of connect reply failed");
            return Err(e.into());
        }
        info!(
            conn = conn.id, %peer_addr, peer = peer_type.name(),
            name = %verified.as_ref().map(|v| v.name.to_string()).unwrap_or_else(|| "-".into()),
            global_id = verified.as_ref().map(|v| v.global_id).unwrap_or(0),
            tag = tag::name(reply.tag), cseq = reply.connect_seq, "accepted"
        );
        conn.st.lock().unwrap().announced = true;
        let _ = conn.events.send(Event::Accepted { conn: conn.handle() }).await;

        if use_seq {
            let acked = match read_array::<8, _>(&mut stream).await {
                Ok(b) => u64::from_le_bytes(b),
                Err(e) => {
                    conn.fault(sgen, "read of acked seq failed");
                    return Err(e);
                }
            };
            let mut st = conn.st.lock().unwrap();
            ConnShared::trim_sent(&mut st, acked);
            debug!(conn = conn.id, acked, in_seq, replay = st.sent.len(), "accept: seq exchanged");
        }
        let (rd, wr) = stream.into_split();
        let signer = conn.st.lock().unwrap().signer.clone();
        let msg_auth = conn_features & features::MSG_AUTH != 0;
        conn.start_io(sgen, V1Reader::new(rd, msg_auth, signer.clone()), V1Writer::new(wr, msg_auth, signer));
        return Ok(());
    }
}

// ---------------------------------------------------------------------------------------
// Connecting side.

/// Run one connection attempt for `conn` (generation `sgen`). On success the socket tasks are
/// running.
pub(crate) async fn connect(msgr: Arc<MsgrShared>, conn: Arc<ConnShared>, sgen: u64) -> Result<(), MsgrError> {
    let (peer, peer_type) = {
        let st = conn.st.lock().unwrap();
        (st.peer_addr, st.peer_type)
    };
    let sock = peer.addr.ok_or_else(|| proto_err("peer has no address"))?;
    let mut stream = TcpStream::connect(sock).await?;
    let _ = stream.set_nodelay(true);
    let my_type = msgr.my_name().ty;

    stream.write_all(BANNER).await?;
    let b: [u8; 9 + 2 * ADDR_LEN] = read_array(&mut stream).await?;
    if &b[..9] != BANNER {
        return Err(proto_err("bad banner from server"));
    }
    let paddr = decode_addr(&b[9..9 + ADDR_LEN])?;
    let _peer_addr_for_me = decode_addr(&b[9 + ADDR_LEN..])?;
    let matches = paddr.addr == peer.addr && paddr.nonce == peer.nonce;
    let blank_ok = is_blank_ip(&paddr)
        && paddr.addr.map(|a| a.port()) == peer.addr.map(|a| a.port())
        && paddr.nonce == peer.nonce;
    if !matches && !blank_ok {
        return Err(proto_err(format!("peer claims to be {paddr}, not {peer}")));
    }
    stream.write_all(&encode_addr(&msgr.my_addr)).await?;

    // No authorizer towards monitors on msgr1 (they authenticate us with MAuth).
    let mut authorizer: Option<ClientAuthorizer> = if peer_type != EntityType::MON { msgr.auth.build_authorizer(peer_type) } else { None };
    if peer_type != EntityType::MON && authorizer.is_none() {
        warn!(peer = peer_type.name(), "connect: no ticket for service; connecting without authorizer");
    }
    let mut global_seq = msgr.next_global_seq(0);
    loop {
        let (connect_seq, lossy) = {
            let st = conn.st.lock().unwrap();
            if st.sgen != sgen || st.phase == Phase::Closed {
                return Err(MsgrError::Closed);
            }
            (st.connect_seq, st.lossy)
        };
        let auth_bl = authorizer.as_ref().map(|a| a.bl.clone()).unwrap_or_default();
        let c = ConnectMsg {
            features: msgr.cfg.features,
            host_type: u32::from(my_type.0),
            global_seq,
            connect_seq,
            protocol_version: proto_version(my_type, peer_type, true),
            authorizer_protocol: if auth_bl.is_empty() { 0 } else { AUTH_CEPHX },
            authorizer_len: auth_bl.len() as u32,
            flags: if lossy { CONNECT_LOSSY } else { 0 },
        };
        let mut out = c.to_bytes().to_vec();
        out.extend_from_slice(&auth_bl);
        stream.write_all(&out).await?;

        let rb: [u8; ConnectReply::LEN] = read_array(&mut stream).await?;
        let r = ConnectReply::from_bytes(&rb);
        if r.authorizer_len > 4096 {
            return Err(proto_err("authorizer reply too long"));
        }
        let auth_reply = read_vec(&mut stream, r.authorizer_len as usize).await?;
        debug!(peer = %peer, tag = tag::name(r.tag), cseq = r.connect_seq, gseq = r.global_seq, "connect: reply");
        if !auth_reply.is_empty()
            && let Some(a) = authorizer.as_mut()
        {
            if r.tag == tag::CHALLENGE_AUTHORIZER {
                a.add_challenge(&auth_reply).map_err(|e| proto_err(format!("challenge: {e}")))?;
                continue;
            }
            a.verify_reply(&auth_reply).map_err(|e| proto_err(format!("authorizer reply: {e}")))?;
        }
        match r.tag {
            tag::FEATURES => return Err(proto_err(format!("feature mismatch, peer requires {:#x}", r.features & !msgr.cfg.features))),
            tag::BADPROTOVER => return Err(proto_err(format!("protocol version mismatch: peer {}", r.protocol_version))),
            tag::BADAUTHORIZER => return Err(proto_err("BADAUTHORIZER")),
            tag::RESETSESSION => {
                info!(peer = %peer, "connect: RESETSESSION");
                let mut st = conn.st.lock().unwrap();
                st.out_q.clear();
                st.sent.clear();
                st.resend_from = 0;
                st.in_seq = 0;
                st.acked_in_seq = 0;
                st.out_seq = 0;
                st.connect_seq = 0;
                continue;
            }
            tag::RETRY_GLOBAL => {
                global_seq = msgr.next_global_seq(r.global_seq);
                continue;
            }
            tag::RETRY_SESSION => {
                conn.st.lock().unwrap().connect_seq = r.connect_seq;
                continue;
            }
            tag::WAIT => return Err(MsgrError::Wait),
            tag::READY | tag::SEQ => {}
            t => return Err(proto_err(format!("unexpected connect reply tag {t}"))),
        }
        let missing = conn.policy.features_required & !r.features;
        if missing != 0 {
            return Err(proto_err(format!("server lacks required features {missing:#x}")));
        }
        if r.tag == tag::SEQ {
            let acked = u64::from_le_bytes(read_array::<8, _>(&mut stream).await?);
            let in_seq = {
                let mut st = conn.st.lock().unwrap();
                ConnShared::trim_sent(&mut st, acked);
                st.in_seq
            };
            stream.write_all(&in_seq.to_le_bytes()).await?;
        }
        // client_ready
        let conn_features = r.features & msgr.cfg.features;
        let signer = authorizer.as_ref().and_then(|a| SessionSigner::new(a.session_key.clone(), conn_features));
        {
            let mut st = conn.st.lock().unwrap();
            if st.sgen != sgen || st.phase == Phase::Closed {
                return Err(MsgrError::Closed);
            }
            st.peer_global_seq = r.global_seq;
            st.lossy = r.flags & CONNECT_LOSSY != 0;
            st.connect_seq += 1;
            st.features = conn_features;
            st.signer = signer.clone();
            st.resend_from = 0;
            st.announced = true;
        }
        info!(conn = conn.id, peer = %peer, peer_type = peer_type.name(), tag = tag::name(r.tag), features = format_args!("{:#x}", conn_features), "connected");
        let (rd, wr) = stream.into_split();
        let msg_auth = conn_features & features::MSG_AUTH != 0;
        let _ = conn.events.send(Event::Connected { conn: conn.handle() }).await;
        conn.start_io(sgen, V1Reader::new(rd, msg_auth, signer.clone()), V1Writer::new(wr, msg_auth, signer));
        return Ok(());
    }
}

// ---------------------------------------------------------------------------------------
// Framing.

/// Reading half of an established msgr1 socket.
pub(crate) struct V1Reader {
    rd: BufReader<OwnedReadHalf>,
    msg_auth: bool,
    signer: Option<SessionSigner>,
}

impl V1Reader {
    pub fn new(rd: OwnedReadHalf, msg_auth: bool, signer: Option<SessionSigner>) -> Self {
        V1Reader { rd: BufReader::with_capacity(64 * 1024, rd), msg_auth, signer }
    }

    async fn read_message(&mut self) -> Result<InFrame, MsgrError> {
        let hb: [u8; MsgHeader::LEN] = read_array(&mut self.rd).await?;
        let h = check_header(&hb)?;
        let front = Bytes::from(read_vec(&mut self.rd, h.front_len as usize).await?);
        let middle = Bytes::from(read_vec(&mut self.rd, h.middle_len as usize).await?);
        let data = Bytes::from(read_vec(&mut self.rd, h.data_len as usize).await?);
        let flen = if self.msg_auth { MsgFooter::LEN } else { MsgFooter::OLD_LEN };
        let fb = read_vec(&mut self.rd, flen).await?;
        let msg = finish_message(&h, front, middle, data, &fb, self.msg_auth, self.signer.as_ref())?;
        Ok(InFrame::Message { seq: h.seq, msg })
    }
}

/// Header crc and size checks.
fn check_header(hb: &[u8; MsgHeader::LEN]) -> Result<MsgHeader, MsgrError> {
    if !MsgHeader::crc_ok(hb) {
        return Err(MsgrError::BadCrc("header"));
    }
    let h = MsgHeader::from_bytes(hb);
    if h.front_len > MAX_SEGMENT || h.middle_len > MAX_SEGMENT || h.data_len > MAX_SEGMENT {
        return Err(proto_err(format!("message too large: {}+{}+{}", h.front_len, h.middle_len, h.data_len)));
    }
    Ok(h)
}

/// Footer checks (`COMPLETE`, segment crcs, signature) and assembly of the message
/// (`ProtocolV1::handle_message_footer`, `decode_message`, `check_message_signature`).
fn finish_message(
    h: &MsgHeader,
    front: Bytes,
    middle: Bytes,
    data: Bytes,
    footer: &[u8],
    msg_auth: bool,
    signer: Option<&SessionSigner>,
) -> Result<RawMessage, MsgrError> {
    let f = MsgFooter::decode(footer, msg_auth);
    if f.flags & FOOTER_COMPLETE == 0 {
        return Err(proto_err("aborted message (footer without COMPLETE)"));
    }
    if ceph_crc32c(0, &front) != f.front_crc {
        return Err(MsgrError::BadCrc("front"));
    }
    if ceph_crc32c(0, &middle) != f.middle_crc {
        return Err(MsgrError::BadCrc("middle"));
    }
    if f.flags & FOOTER_NOCRC == 0 && ceph_crc32c(0, &data) != f.data_crc {
        return Err(MsgrError::BadCrc("data"));
    }
    if let Some(s) = signer
        && s.checks()
    {
        let sig = s
            .signature(&SigFields {
                header_crc: h.crc,
                front_crc: f.front_crc,
                front_len: h.front_len,
                middle_crc: f.middle_crc,
                middle_len: h.middle_len,
                data_crc: f.data_crc,
                data_len: h.data_len,
                seq: h.seq,
            })
            .map_err(|e| proto_err(format!("signature: {e}")))?;
        if sig != f.sig {
            return Err(MsgrError::BadSignature { seq: h.seq });
        }
    }
    Ok(RawMessage {
        msg_type: h.msg_type,
        version: h.version,
        compat_version: h.compat_version,
        priority: h.priority,
        tid: h.tid,
        data_off: h.data_off,
        src: Some(h.src),
        front,
        middle,
        data,
    })
}

/// Encode one complete msgr1 `MSG` frame (tag, header with crc, segments, footer with crcs
/// and, given a signer, the cephx signature), exactly as the socket writer does.
pub fn encode_msg_frame(out: &mut Vec<u8>, seq: u64, m: &RawMessage, src: EntityName, msg_auth: bool, signer: Option<&SessionSigner>) -> Result<(), MsgrError> {
    V1Writer::encode_message(out, seq, m, src, msg_auth, signer)
}

/// Decode and verify one complete msgr1 `MSG` frame held in memory (tag byte, header,
/// segments, footer): the same checks as on a socket. Returns the sequence number and the
/// message. For tools and tests that replay captured traffic.
pub fn decode_msg_frame(frame: &[u8], msg_auth: bool, signer: Option<&SessionSigner>) -> Result<(u64, RawMessage), MsgrError> {
    let short = || proto_err("short frame");
    if frame.first() != Some(&tag::MSG) {
        return Err(proto_err("not a MSG frame"));
    }
    let hb: &[u8; MsgHeader::LEN] = frame.get(1..1 + MsgHeader::LEN).ok_or_else(short)?.try_into().unwrap();
    let h = check_header(hb)?;
    let mut off = 1 + MsgHeader::LEN;
    let mut seg = |n: u32| -> Result<Bytes, MsgrError> {
        let s = frame.get(off..off + n as usize).ok_or_else(short)?;
        off += n as usize;
        Ok(Bytes::copy_from_slice(s))
    };
    let front = seg(h.front_len)?;
    let middle = seg(h.middle_len)?;
    let data = seg(h.data_len)?;
    let flen = if msg_auth { MsgFooter::LEN } else { MsgFooter::OLD_LEN };
    let footer = frame.get(off..off + flen).ok_or_else(short)?;
    let msg = finish_message(&h, front, middle, data, footer, msg_auth, signer)?;
    Ok((h.seq, msg))
}

impl FrameReader for V1Reader {
    async fn read_frame(&mut self) -> Result<InFrame, MsgrError> {
        let t = self.rd.read_u8().await?;
        match t {
            tag::MSG => self.read_message().await,
            tag::ACK => Ok(InFrame::Ack(self.rd.read_u64_le().await?)),
            tag::KEEPALIVE => Ok(InFrame::Keepalive),
            tag::KEEPALIVE2 => {
                let sec = self.rd.read_u32_le().await?;
                let nsec = self.rd.read_u32_le().await?;
                Ok(InFrame::Keepalive2(UTime { sec, nsec }))
            }
            tag::KEEPALIVE2_ACK => {
                let sec = self.rd.read_u32_le().await?;
                let nsec = self.rd.read_u32_le().await?;
                Ok(InFrame::Keepalive2Ack(UTime { sec, nsec }))
            }
            tag::CLOSE => Ok(InFrame::Close),
            t => Err(proto_err(format!("bad tag {t}"))),
        }
    }
}

/// Writing half of an established msgr1 socket.
pub(crate) struct V1Writer {
    wr: OwnedWriteHalf,
    msg_auth: bool,
    signer: Option<SessionSigner>,
    buf: Vec<u8>,
}

impl V1Writer {
    pub fn new(wr: OwnedWriteHalf, msg_auth: bool, signer: Option<SessionSigner>) -> Self {
        V1Writer { wr, msg_auth, signer, buf: Vec::with_capacity(64 * 1024) }
    }

    /// Encode one message (`ProtocolV1::write_message`).
    pub(crate) fn encode_message(buf: &mut Vec<u8>, seq: u64, m: &RawMessage, src: EntityName, msg_auth: bool, signer: Option<&SessionSigner>) -> Result<(), MsgrError> {
        let mut footer = MsgFooter {
            front_crc: ceph_crc32c(0, &m.front),
            middle_crc: ceph_crc32c(0, &m.middle),
            data_crc: ceph_crc32c(0, &m.data),
            sig: 0,
            flags: FOOTER_COMPLETE,
        };
        let mut h = MsgHeader {
            seq,
            tid: m.tid,
            msg_type: m.msg_type,
            priority: m.priority,
            version: m.version,
            front_len: m.front.len() as u32,
            middle_len: m.middle.len() as u32,
            data_len: m.data.len() as u32,
            data_off: m.data_off,
            src,
            // Message::encode: an unset compat version means "same as version".
            compat_version: if m.compat_version == 0 { m.version } else { m.compat_version },
            reserved: 0,
            crc: 0,
        };
        let hb = h.to_bytes();
        if let Some(s) = signer {
            footer.sig = s
                .signature(&SigFields {
                    header_crc: h.crc,
                    front_crc: footer.front_crc,
                    front_len: h.front_len,
                    middle_crc: footer.middle_crc,
                    middle_len: h.middle_len,
                    data_crc: footer.data_crc,
                    data_len: h.data_len,
                    seq,
                })
                .map_err(|e| proto_err(format!("signing: {e}")))?;
            footer.flags |= FOOTER_SIGNED;
        }
        buf.push(tag::MSG);
        buf.extend_from_slice(&hb);
        buf.extend_from_slice(&m.front);
        buf.extend_from_slice(&m.middle);
        buf.extend_from_slice(&m.data);
        footer.encode(msg_auth, buf);
        Ok(())
    }
}

impl FrameWriter for V1Writer {
    async fn write_frames(&mut self, frames: &[OutFrame], src: EntityName) -> Result<(), MsgrError> {
        self.buf.clear();
        for f in frames {
            match f {
                OutFrame::Message { seq, msg } => {
                    Self::encode_message(&mut self.buf, *seq, msg, src, self.msg_auth, self.signer.as_ref())?;
                }
                OutFrame::Ack(seq) => {
                    self.buf.push(tag::ACK);
                    self.buf.extend_from_slice(&seq.to_le_bytes());
                }
                OutFrame::Keepalive => self.buf.push(tag::KEEPALIVE),
                OutFrame::Keepalive2(t) => {
                    self.buf.push(tag::KEEPALIVE2);
                    self.buf.extend_from_slice(&t.sec.to_le_bytes());
                    self.buf.extend_from_slice(&t.nsec.to_le_bytes());
                }
                OutFrame::Keepalive2Ack(t) => {
                    self.buf.push(tag::KEEPALIVE2_ACK);
                    self.buf.extend_from_slice(&t.sec.to_le_bytes());
                    self.buf.extend_from_slice(&t.nsec.to_le_bytes());
                }
                OutFrame::Close => self.buf.push(tag::CLOSE),
            }
        }
        self.wr.write_all(&self.buf).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auth::CryptoKey;

    #[test]
    fn encoded_message_layout() {
        let mut m = RawMessage::new(22, 7, 1, Bytes::from_static(b"hello"));
        m.tid = 9;
        let mut buf = Vec::new();
        let key = CryptoKey::new_aes(*b"0123456789abcdef", UTime::default());
        let signer = SessionSigner::new(key, features::ALL).unwrap();
        V1Writer::encode_message(&mut buf, 3, &m, EntityName::new(EntityType::MDS, 0), true, Some(&signer)).unwrap();
        assert_eq!(buf.len(), 1 + 53 + 5 + 21);
        assert_eq!(buf[0], tag::MSG);
        let hb: [u8; 53] = buf[1..54].try_into().unwrap();
        assert!(MsgHeader::crc_ok(&hb));
        let h = MsgHeader::from_bytes(&hb);
        assert_eq!((h.seq, h.tid, h.msg_type, h.version, h.compat_version, h.front_len), (3, 9, 22, 7, 1, 5));
        let f = MsgFooter::decode(&buf[59..], true);
        assert_eq!(f.flags, FOOTER_COMPLETE | FOOTER_SIGNED);
        assert_eq!(f.front_crc, ceph_crc32c(0, b"hello"));
        let expect = signer
            .signature(&SigFields { header_crc: h.crc, front_crc: f.front_crc, front_len: 5, seq: 3, ..Default::default() })
            .unwrap();
        assert_eq!(f.sig, expect);
    }
}
