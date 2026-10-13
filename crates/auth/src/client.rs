//! cephx principal side: authenticating to the monitors, service tickets and authorizers.
//!
//! Follows `src/auth/cephx/CephxClientHandler.cc` (`build_request`, `handle_response`,
//! `build_rotating_request`) and `src/auth/cephx/CephxProtocol.cc` (`CephXTicketHandler`,
//! `CephXTicketManager`, `CephXAuthorizer`, `cephx_calc_client_server_challenge`).
//!
//! Exchange with a monitor (all inside `MAuth`/`MAuthReply` payloads on msgr1):
//!
//! 1. reply 1 carries `CephXServerChallenge {u8 1, u64 server_challenge}`;
//! 2. request `CEPHX_GET_AUTH_SESSION_KEY`: `u16 0x0100` + `CephXAuthenticate` whose `key` is the
//!    challenge proof and whose `old_ticket` is the previous auth ticket (global id reclaim);
//!    the reply holds the auth ticket (`CephXResponseHeader` + service ticket reply);
//! 3. request `CEPHX_GET_PRINCIPAL_SESSION_KEY`: `u16 0x0200` + an authorizer built from the auth
//!    ticket + `CephXServiceTicketRequest {u8 1, u32 keys}`; the reply holds one service ticket
//!    per requested type, sealed with the auth session key;
//! 4. request `CEPHX_GET_ROTATING_KEY`: `u16 0x0400`; the reply is
//!    `decode_decrypt(RotatingSecrets)` under the principal's own key.
//!
//! A service ticket reply is `u8 1`, `u32 n`, then n x (`u32 service_id`, `u8 1`,
//! `encode_encrypt(CephXServiceTicket)`, `u8 ticket_enc`, ticket blob — either a plain
//! `bufferlist` or, when `ticket_enc`, `encode_encrypt(bufferlist)` under the old session key).

use std::collections::BTreeMap;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use denc::{Decode, Encode, UTime};

use crate::crypto::{
    decode_decrypt, decode_decrypt_value, decrypt_enc_bl_value, encode_encrypt,
    encode_encrypt_value,
};
use crate::types::*;
use crate::{AuthError, AuthName, CryptoKey, usage};
use crate::{ENTITY_AUTH, ENTITY_MGR, utime_add_secs, utime_now};

/// `CEPHX_GET_AUTH_SESSION_KEY`.
pub const CEPHX_GET_AUTH_SESSION_KEY: u16 = 0x0100;
/// `CEPHX_GET_PRINCIPAL_SESSION_KEY`.
pub const CEPHX_GET_PRINCIPAL_SESSION_KEY: u16 = 0x0200;
/// `CEPHX_GET_ROTATING_KEY`.
pub const CEPHX_GET_ROTATING_KEY: u16 = 0x0400;

/// `cephx_calc_client_server_challenge`. For AES keys: `encode_encrypt` of
/// `CephXChallengeBlob {u64 server, u64 client}` (no struct_v), then XOR of every complete
/// little-endian 64-bit word of the result, which includes the `u32` length prefix of
/// `encode_encrypt` exactly as the C++ code folds `enc.c_str()`. For AES256KRB5 keys
/// (`encode_hash`): HMAC-SHA256 of the 16-byte blob keyed with the secret, folded the same way.
pub fn calc_client_server_challenge(
    secret: &CryptoKey,
    server_challenge: u64,
    client_challenge: u64,
) -> Result<u64, AuthError> {
    let mut blob = BytesMut::with_capacity(16);
    blob.put_u64_le(server_challenge);
    blob.put_u64_le(client_challenge);
    let mut enc = BytesMut::new();
    if secret.key_type() == crate::crypto::CRYPTO_AES {
        encode_encrypt(&mut enc, secret, 0, &blob)?;
    } else {
        enc.put_slice(&secret.hmac_sha256(&blob));
    }
    let mut k = 0u64;
    for w in enc.as_chunks::<8>().0 {
        k ^= u64::from_le_bytes(*w);
    }
    Ok(k)
}

/// `CephXTicketHandler`: the session key and opaque ticket for one service.
#[derive(Debug, Clone)]
pub struct TicketHandler {
    pub service_id: u32,
    pub session_key: CryptoKey,
    pub ticket: CephXTicketBlob,
    pub renew_after: UTime,
    pub expires: UTime,
    have_key_flag: bool,
}

impl TicketHandler {
    fn new(service_id: u32) -> Self {
        TicketHandler {
            service_id,
            session_key: CryptoKey::default(),
            ticket: CephXTicketBlob::default(),
            renew_after: UTime::default(),
            expires: UTime::default(),
            have_key_flag: false,
        }
    }

    /// `CephXTicketHandler::verify_service_ticket_reply` for one entry (after the service id).
    fn verify_service_ticket_reply(
        &mut self,
        secret: &CryptoKey,
        buf: &mut Bytes,
    ) -> Result<(), AuthError> {
        let _service_ticket_v = u8::decode(buf)?;
        let msg_a: CephXServiceTicket =
            decode_decrypt_value(buf, secret, usage::TICKET_SESSION_KEY)?;
        let ticket_enc = u8::decode(buf)?;
        let mut service_ticket_bl = if ticket_enc != 0 {
            // Sealed with the session key this handler held so far (the old auth ticket).
            let mut p = decode_decrypt(buf, &self.session_key, usage::TICKET_BLOB)?;
            Bytes::decode(&mut p)?
        } else {
            Bytes::decode(buf)?
        };
        self.ticket = CephXTicketBlob::decode(&mut service_ticket_bl)?;
        self.session_key = msg_a.session_key;
        if msg_a.validity != UTime::default() {
            let now = utime_now();
            let validity = f64::from(msg_a.validity.sec) + f64::from(msg_a.validity.nsec) * 1e-9;
            self.expires = utime_add_secs(now, validity);
            self.renew_after = utime_add_secs(self.expires, -(f64::from(msg_a.validity.sec) / 4.0));
        }
        self.have_key_flag = true;
        Ok(())
    }

    /// `have_key`: a key that has not expired.
    pub fn have_key(&mut self) -> bool {
        if self.have_key_flag {
            self.have_key_flag = utime_now() < self.expires;
        }
        self.have_key_flag
    }

    /// `need_key`: no key, or the renewal time passed.
    pub fn need_key(&self) -> bool {
        if self.have_key_flag {
            return self.expires != UTime::default() && utime_now() >= self.renew_after;
        }
        true
    }

    /// `CephXTicketHandler::build_authorizer`: `u8 1` (authorizer_v / `AUTH_MODE_AUTHORIZER`),
    /// `u64 global_id`, `u32 service_id`, `CephXTicketBlob`, then
    /// `encode_encrypt(CephXAuthorize {nonce})` under the ticket's session key.
    pub fn build_authorizer(&self, global_id: u64) -> Result<ClientAuthorizer, AuthError> {
        let nonce: u64 = rand::random();
        let mut base = BytesMut::new();
        base.put_u8(1);
        base.put_u64_le(global_id);
        base.put_u32_le(self.service_id);
        self.ticket.encode(&mut base, 0);
        let base = base.freeze();
        let mut bl = BytesMut::from(&base[..]);
        encode_encrypt_value(
            &mut bl,
            &self.session_key,
            usage::AUTHORIZE,
            &CephXAuthorize {
                nonce,
                ..Default::default()
            },
        )?;
        Ok(ClientAuthorizer {
            protocol: crate::AUTH_CEPHX,
            bl: bl.freeze(),
            base,
            session_key: self.session_key.clone(),
            nonce,
        })
    }
}

/// An authorizer for an outgoing connection (`CephXAuthorizer`).
#[derive(Debug, Clone)]
pub struct ClientAuthorizer {
    /// `CEPH_AUTH_CEPHX`.
    pub protocol: u32,
    /// The payload to send (initial, or rebuilt after a challenge).
    pub bl: Bytes,
    base: Bytes,
    /// The ticket session key; signs messages on the connection.
    pub session_key: CryptoKey,
    nonce: u64,
}

impl ClientAuthorizer {
    /// `CephXAuthorizer::add_challenge`: decrypt the server's `CephXAuthorizeChallenge` (raw
    /// ciphertext, no length prefix) and rebuild the payload with `server_challenge + 1`.
    pub fn add_challenge(&mut self, challenge: &[u8]) -> Result<(), AuthError> {
        let mut msg = CephXAuthorize {
            nonce: self.nonce,
            ..Default::default()
        };
        if !challenge.is_empty() {
            let ch: CephXChallenge =
                decrypt_enc_bl_value(&self.session_key, usage::AUTHORIZE_CHALLENGE, challenge)?;
            msg.have_challenge = true;
            msg.server_challenge_plus_one = ch.server_challenge.wrapping_add(1);
        }
        let mut bl = BytesMut::from(&self.base[..]);
        encode_encrypt_value(&mut bl, &self.session_key, usage::AUTHORIZE, &msg)?;
        self.bl = bl.freeze();
        Ok(())
    }

    /// `CephXAuthorizer::verify_reply`: decrypt `CephXAuthorizeReply` and check `nonce + 1`.
    /// Returns the connection secret (empty on msgr1).
    pub fn verify_reply(&self, reply: &[u8]) -> Result<Bytes, AuthError> {
        let mut b = Bytes::copy_from_slice(reply);
        let r: CephXAuthorizeReply =
            decode_decrypt_value(&mut b, &self.session_key, usage::AUTHORIZE_REPLY)?;
        if r.nonce_plus_one != self.nonce.wrapping_add(1) {
            return Err(AuthError::Rejected(format!(
                "authorizer reply nonce {} != {}",
                r.nonce_plus_one,
                self.nonce.wrapping_add(1)
            )));
        }
        Ok(r.connection_secret)
    }
}

/// Outcome of [`CephxClient::handle_response`].
#[derive(Debug)]
pub enum Progress {
    /// Authenticated and all wanted tickets held (C++ return 0).
    Done,
    /// Send [`CephxClient::build_request`] next (C++ `-EAGAIN`).
    Continue,
    /// A rotating secrets reply.
    Rotating(RotatingSecrets),
}

/// The cephx client state of one principal (`CephxClientHandler` + `CephXTicketManager`).
/// It survives monitor sessions: tickets are kept for global id reclaim.
#[derive(Debug)]
pub struct CephxClient {
    name: AuthName,
    secret: CryptoKey,
    global_id: u64,
    want: u32,
    have: u32,
    need: u32,
    tickets: BTreeMap<u32, TicketHandler>,
    starting: bool,
    server_challenge: u64,
}

impl CephxClient {
    /// `want_keys` is a mask of `CEPH_ENTITY_TYPE_*` service types; the auth ticket is always
    /// wanted (`AuthClientHandler::set_want_keys`).
    pub fn new(name: AuthName, secret: CryptoKey, want_keys: u32) -> Self {
        let mut c = CephxClient {
            name,
            secret,
            global_id: 0,
            want: want_keys | ENTITY_AUTH,
            have: 0,
            need: 0,
            tickets: BTreeMap::new(),
            starting: true,
            server_challenge: 0,
        };
        c.validate_tickets();
        c
    }

    pub fn name(&self) -> &AuthName {
        &self.name
    }

    pub fn global_id(&self) -> u64 {
        self.global_id
    }

    pub fn set_global_id(&mut self, id: u64) {
        self.global_id = id;
    }

    /// Start a new session: the next reply is a server challenge (`CephxClientHandler::reset`).
    pub fn reset(&mut self) {
        self.starting = true;
        self.server_challenge = 0;
    }

    /// Drop all tickets (`invalidate_all_tickets`), e.g. when the MonMap `auth_epoch` grows.
    pub fn invalidate_all_tickets(&mut self) {
        for t in self.tickets.values_mut() {
            t.have_key_flag = false;
        }
    }

    /// The ticket handler of a service, if any.
    pub fn ticket(&self, service_id: u32) -> Option<&TicketHandler> {
        self.tickets.get(&service_id)
    }

    fn handler(&mut self, service_id: u32) -> &mut TicketHandler {
        self.tickets
            .entry(service_id)
            .or_insert_with(|| TicketHandler::new(service_id))
    }

    /// `CephXTicketManager::validate_tickets(want, have, need)`.
    fn validate_tickets(&mut self) {
        let mut have = self.have;
        let mut need = 0u32;
        let mut i = 1u32;
        while i != 0 && i <= self.want {
            if self.want & i != 0 {
                match self.tickets.get_mut(&i) {
                    None => {
                        have &= !i;
                        need |= i;
                    }
                    Some(t) => {
                        if t.need_key() {
                            need |= i;
                        } else {
                            need &= !i;
                        }
                        if t.have_key() {
                            have |= i;
                        } else {
                            have &= !i;
                        }
                    }
                }
            }
            i <<= 1;
        }
        self.have = have;
        self.need = need;
    }

    /// `_need_tickets`: anything but only the MGR ticket.
    fn need_tickets_inner(&self) -> bool {
        self.need != 0 && self.need != ENTITY_MGR
    }

    /// `need_tickets`: re-validate and tell whether a ticket request is due (renewal).
    pub fn need_tickets(&mut self) -> bool {
        self.validate_tickets();
        self.need_tickets_inner()
    }

    /// Mask of service types whose ticket is valid.
    pub fn have(&mut self) -> u32 {
        self.validate_tickets();
        self.have
    }

    /// `prepare_build_request` + `build_request`: the next request payload.
    pub fn build_request(&mut self) -> Result<Bytes, AuthError> {
        self.validate_tickets();
        let mut bl = BytesMut::new();
        if self.need & ENTITY_AUTH != 0 {
            encode_request_header(&mut bl, CEPHX_GET_AUTH_SESSION_KEY);
            if self.secret.is_empty() {
                return Err(AuthError::Protocol(format!("no secret for {}", self.name)));
            }
            let client_challenge: u64 = rand::random();
            let key = calc_client_server_challenge(
                &self.secret,
                self.server_challenge,
                client_challenge,
            )?;
            let old_ticket = self.handler(ENTITY_AUTH).ticket.clone();
            let req = CephXAuthenticate {
                client_challenge,
                key,
                old_ticket,
                other_keys: self.need,
            };
            req.encode(&mut bl, 0);
            return Ok(bl.freeze());
        }
        if self.need_tickets_inner() {
            encode_request_header(&mut bl, CEPHX_GET_PRINCIPAL_SESSION_KEY);
            let global_id = self.global_id;
            let auth = self.handler(ENTITY_AUTH).build_authorizer(global_id)?;
            bl.put_slice(&auth.bl);
            // CephXServiceTicketRequest
            bl.put_u8(1);
            bl.put_u32_le(self.need);
        }
        Ok(bl.freeze())
    }

    /// `build_rotating_request`: `u16 CEPHX_GET_ROTATING_KEY`.
    pub fn build_rotating_request(&self) -> Bytes {
        let mut bl = BytesMut::new();
        encode_request_header(&mut bl, CEPHX_GET_ROTATING_KEY);
        bl.freeze()
    }

    /// `CephXTicketManager::verify_service_ticket_reply`.
    fn verify_service_ticket_reply(
        &mut self,
        secret: &CryptoKey,
        buf: &mut Bytes,
    ) -> Result<(), AuthError> {
        let _v = u8::decode(buf)?;
        let num = u32::decode(buf)?;
        for _ in 0..num {
            let ty = u32::decode(buf)?;
            let h = self.handler(ty);
            h.verify_service_ticket_reply(secret, buf)?;
            h.service_id = ty;
        }
        Ok(())
    }

    /// `CephxClientHandler::handle_response` for a msgr1 `MAuthReply` (`result`, `result_bl`).
    pub fn handle_response(
        &mut self,
        result: i32,
        mut payload: Bytes,
    ) -> Result<Progress, AuthError> {
        if result < 0 {
            return Err(AuthError::Server(result));
        }
        if self.starting {
            let ch = CephXChallenge::decode(&mut payload)?;
            self.server_challenge = ch.server_challenge;
            self.starting = false;
            if let Some(t) = self.tickets.get_mut(&ENTITY_AUTH) {
                t.have_key_flag = false; // invalidate_ticket(AUTH)
            }
            return Ok(Progress::Continue);
        }
        let header = CephXResponseHeader::decode(&mut payload)?;
        match header.request_type {
            CEPHX_GET_AUTH_SESSION_KEY => {
                let secret = self.secret.clone();
                self.verify_service_ticket_reply(&secret, &mut payload)?;
                // msgr1: the connection secret and extra tickets that may follow are ignored,
                // and the service tickets are fetched with CEPHX_GET_PRINCIPAL_SESSION_KEY, as
                // the C++ client does when it has no session key/connection secret pointers.
                self.validate_tickets();
                if self.need_tickets_inner() {
                    Ok(Progress::Continue)
                } else {
                    Ok(Progress::Done)
                }
            }
            CEPHX_GET_PRINCIPAL_SESSION_KEY => {
                let key = self.handler(ENTITY_AUTH).session_key.clone();
                self.verify_service_ticket_reply(&key, &mut payload)?;
                self.validate_tickets();
                Ok(Progress::Done)
            }
            CEPHX_GET_ROTATING_KEY => {
                let secrets: RotatingSecrets =
                    decode_decrypt_value(&mut payload, &self.secret, usage::ROTATING_SECRET)?;
                Ok(Progress::Rotating(secrets))
            }
            t => Err(AuthError::Protocol(format!(
                "unknown cephx response type {t:#x}"
            ))),
        }
    }

    /// `CephXTicketManager::build_authorizer` for a connection to a service.
    pub fn build_authorizer(&self, service_id: u32) -> Option<ClientAuthorizer> {
        let h = self.tickets.get(&service_id)?;
        if !h.have_key_flag {
            return None;
        }
        h.build_authorizer(self.global_id).ok()
    }
}

/// The msgr1 `MAuth` payload that opens a monitor session with protocol `CEPH_AUTH_UNKNOWN`
/// (`MonConnection::start`): `u8 1`, `vector<u32>` supported methods, `EntityName`,
/// `u64 global_id`.
pub fn initial_mauth_payload(name: &AuthName, global_id: u64, methods: &[u32]) -> Bytes {
    let mut b = BytesMut::new();
    b.put_u8(1);
    methods.to_vec().encode(&mut b, 0);
    name.encode(&mut b, 0);
    b.put_u64_le(global_id);
    b.freeze()
}

/// Skip helper used by tests: remaining length of a payload.
#[allow(dead_code)]
fn remaining(b: &Bytes) -> usize {
    b.remaining()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::{self, MonTicketIssuer};
    use crate::{ENTITY_MDS, ENTITY_MON, ENTITY_OSD};
    use denc::EntityType;

    #[test]
    fn challenge_proof_folds_length_prefix() {
        // Independent computation: AES-CBC(key, {u8 1, magic, server, client} + pad) with the
        // u32 length (32) in front, folded into 64-bit words.
        let k = CryptoKey::new_aes(*b"0123456789abcdef", UTime::default());
        let mut plain = vec![1u8];
        plain.extend_from_slice(&crate::crypto::AUTH_ENC_MAGIC.to_le_bytes());
        plain.extend_from_slice(&7u64.to_le_bytes());
        plain.extend_from_slice(&9u64.to_le_bytes());
        let ct = k.encrypt(0, &plain).unwrap();
        assert_eq!(ct.len(), 32);
        let mut enc = 32u32.to_le_bytes().to_vec();
        enc.extend_from_slice(&ct);
        let mut expect = 0u64;
        for w in enc.as_chunks::<8>().0 {
            expect ^= u64::from_le_bytes(*w);
        }
        assert_eq!(calc_client_server_challenge(&k, 7, 9).unwrap(), expect);
    }

    #[test]
    fn challenge_proof_aes256krb5_is_hmac_sha256() {
        // Reference: Python hmac/hashlib, key bytes 0..32, blob {7, 9}.
        let secret: Vec<u8> = (0..32u8).collect();
        let k =
            CryptoKey::new(crate::crypto::CRYPTO_AES256KRB5, UTime::default(), &secret).unwrap();
        assert_eq!(
            calc_client_server_challenge(&k, 7, 9).unwrap(),
            0xb078_64f3_8c12_e567
        );
    }

    /// Full monitor exchange against the test-only ticket issuer in `service`.
    #[test]
    fn handshake_against_issuer() {
        handshake_with_key(CryptoKey::generate_aes(utime_now()));
    }

    /// The same exchange with an AES256KRB5 principal key (Ceph after 20.2): every envelope
    /// uses its key usage.
    #[test]
    fn handshake_against_issuer_aes256krb5() {
        handshake_with_key(CryptoKey::generate_aes256krb5(utime_now()));
    }

    fn handshake_with_key(key: CryptoKey) {
        let name = AuthName::new(EntityType::MDS, "a");
        let mut mon = MonTicketIssuer::new(name.clone(), key.clone(), 4242);
        let mut c = CephxClient::new(
            name.clone(),
            key,
            ENTITY_MON | ENTITY_OSD | ENTITY_MDS | ENTITY_MGR,
        );

        // 1: server challenge.
        let p = c.handle_response(0, mon.server_challenge()).unwrap();
        assert!(matches!(p, Progress::Continue));
        c.set_global_id(4242);
        // 2: auth session key.
        let req = c.build_request().unwrap();
        let reply = mon.handle_request(req).unwrap();
        assert!(matches!(
            c.handle_response(0, reply).unwrap(),
            Progress::Continue
        ));
        // 3: service tickets.
        let req = c.build_request().unwrap();
        let reply = mon.handle_request(req).unwrap();
        assert!(matches!(
            c.handle_response(0, reply).unwrap(),
            Progress::Done
        ));
        assert!(!c.need_tickets());
        assert_eq!(
            c.have() & (ENTITY_MON | ENTITY_MDS),
            ENTITY_MON | ENTITY_MDS
        );
        // 4: rotating secrets.
        let reply = mon.handle_request(c.build_rotating_request()).unwrap();
        match c.handle_response(0, reply).unwrap() {
            Progress::Rotating(r) => assert_eq!(r.secrets.len(), 3),
            p => panic!("unexpected {p:?}"),
        }
        // An authorizer for the MDS service verifies against the service's rotating secrets.
        let mut a = c.build_authorizer(ENTITY_MDS).unwrap();
        let ks = mon.mds_keystore();
        let mut ch = None;
        let out = service::verify_authorizer(&ks, ENTITY_MDS, &a.bl, Some(&mut ch), 0).unwrap();
        let service::VerifyOutcome::Challenge { reply } = out else {
            panic!("expected challenge")
        };
        a.add_challenge(&reply).unwrap();
        let out = service::verify_authorizer(&ks, ENTITY_MDS, &a.bl, Some(&mut ch), 0).unwrap();
        let service::VerifyOutcome::Accepted(v) = out else {
            panic!("expected accept")
        };
        assert_eq!(v.global_id, 4242);
        assert_eq!(v.name, name);
        a.verify_reply(&v.reply).unwrap();
    }
}
