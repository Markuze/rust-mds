//! cephx service side: verifying the authorizers that peers present to this daemon.
//!
//! Follows `cephx_verify_authorizer` (`src/auth/cephx/CephxProtocol.cc`),
//! `CephxAuthorizeHandler::verify_authorizer` and the challenge logic of
//! `MonClient::handle_auth_request` (`src/mon/MonClient.cc`).
//!
//! Authorizer layout: `u8 authorizer_v` (1), `u64 global_id`, `u32 service_id`,
//! `CephXTicketBlob` (sealed `CephXServiceTicketInfo` under the rotating service secret
//! `secret_id`, usage `TICKET_INFO`), then `encode_encrypt(CephXAuthorize)` under the ticket
//! session key (usage `AUTHORIZE`).
//!
//! With a challenge (CEPHX_V2 peers), a first authorizer without `have_challenge` is answered
//! with the raw ciphertext of `CephXAuthorizeChallenge {u8 1, u64 server_challenge}` (usage
//! `AUTHORIZE_CHALLENGE`, no length prefix); the peer resends with `server_challenge + 1`.
//! The final reply is `encode_encrypt(CephXAuthorizeReply {nonce + 1, connection_secret})`
//! (usage `AUTHORIZE_REPLY`, `u32` length prefix).

use std::collections::BTreeMap;

use bytes::{BufMut, Bytes, BytesMut};
use denc::{Decode, Encode, UTime};

use crate::crypto::{decode_decrypt_value, decrypt_enc_bl_value, encode_encrypt, encode_encrypt_value, encrypt_enc_bl_value};
use crate::types::*;
use crate::{AuthError, AuthName, CryptoKey, usage};

/// Where a service finds the secrets that seal tickets (`KeyStore`).
pub trait KeyStore {
    /// Long-term secret of a principal (used for `secret_id == u64::MAX`).
    fn get_secret(&self, name: &AuthName) -> Option<CryptoKey>;
    /// Rotating secret `secret_id` of service type `service_id`.
    fn get_service_secret(&self, service_id: u32, secret_id: u64) -> Option<CryptoKey>;
}

/// The rotating secrets of this daemon's own service type (`RotatingKeyRing`): it refuses any
/// other service type.
#[derive(Debug, Clone, Default)]
pub struct ServiceKeys {
    pub service_id: u32,
    pub rotating: RotatingSecrets,
}

impl KeyStore for ServiceKeys {
    fn get_secret(&self, _name: &AuthName) -> Option<CryptoKey> {
        None
    }
    fn get_service_secret(&self, service_id: u32, secret_id: u64) -> Option<CryptoKey> {
        if service_id != self.service_id {
            return None;
        }
        self.rotating.get(secret_id).cloned()
    }
}

/// A verified peer.
#[derive(Debug, Clone)]
pub struct VerifiedAuthorizer {
    /// The authenticated principal (`peer_name`).
    pub name: AuthName,
    /// The authenticated global id (`peer_global_id`).
    pub global_id: u64,
    /// The caps blob for this service (`peer_caps_info`).
    pub caps: AuthCapsInfo,
    /// The ticket session key: signs messages on msgr1.
    pub session_key: CryptoKey,
    /// The encrypted `CephXAuthorizeReply` to return (`u32` length + ciphertext).
    pub reply: Bytes,
    pub connection_secret: Bytes,
}

/// Result of [`verify_authorizer`].
#[derive(Debug)]
pub enum VerifyOutcome {
    Accepted(Box<VerifiedAuthorizer>),
    /// Send this challenge (raw ciphertext) and wait for the authorizer again.
    Challenge { reply: Bytes },
}

/// `cephx_verify_authorizer`. `challenge` is `None` to skip the challenge round (peer without
/// CEPHX_V2), else the per-connection challenge state.
pub fn verify_authorizer(
    keys: &dyn KeyStore,
    expected_service: u32,
    payload: &[u8],
    challenge: Option<&mut Option<u64>>,
    connection_secret_len: usize,
) -> Result<VerifyOutcome, AuthError> {
    let mut p = Bytes::copy_from_slice(payload);
    let _authorizer_v = u8::decode(&mut p)?;
    let global_id = u64::decode(&mut p)?;
    let service_id = u32::decode(&mut p)?;
    let ticket = CephXTicketBlob::decode(&mut p)?;
    if service_id != expected_service {
        return Err(AuthError::Rejected(format!("authorizer for service {service_id:#x}, I am {expected_service:#x}")));
    }

    let service_secret = if ticket.secret_id == u64::MAX {
        let mut n = AuthName::new(denc::EntityType(service_id as u8), "");
        n.id.clear();
        keys.get_secret(&n)
    } else {
        keys.get_service_secret(service_id, ticket.secret_id)
    }
    .ok_or_else(|| AuthError::Rejected(format!("no service secret {} for service {service_id:#x}", ticket.secret_id)))?;
    if service_secret.is_empty() {
        return Err(AuthError::Rejected("invalid service key".into()));
    }

    let info: CephXServiceTicketInfo = decrypt_enc_bl_value(&service_secret, usage::TICKET_INFO, &ticket.blob)
        .map_err(|e| AuthError::Rejected(format!("could not decrypt ticket info: {e}")))?;

    if info.ticket.global_id != global_id {
        return Err(AuthError::Rejected(format!("global_id mismatch: declared {global_id}, ticket {}", info.ticket.global_id)));
    }
    if info.ticket.auid != AUTH_UID_DEFAULT {
        return Err(AuthError::Rejected("bad auid in ticket".into()));
    }
    if info.ticket.caps.allow_all {
        return Err(AuthError::Rejected("allow_all set in ticket".into()));
    }
    check_ticket_timestamps(&info)?;
    if !info.ticket.caps.caps.is_empty() && info.ticket.caps.cap_string().is_none() {
        return Err(AuthError::Rejected("caps blob is not exactly one string".into()));
    }
    if info.session_key.is_empty() {
        return Err(AuthError::Rejected("session key is invalid".into()));
    }

    let auth_msg: CephXAuthorize = decode_decrypt_value(&mut p, &info.session_key, usage::AUTHORIZE)
        .map_err(|e| AuthError::Rejected(format!("could not decrypt authorize request: {e}")))?;

    if let Some(ch) = challenge {
        match *ch {
            Some(c) if auth_msg.have_challenge => {
                if c.wrapping_add(1) != auth_msg.server_challenge_plus_one {
                    return Err(AuthError::Rejected("bad server_challenge + 1".into()));
                }
            }
            _ => {
                let c: u64 = rand::random();
                *ch = Some(c);
                let reply = encrypt_enc_bl_value(&info.session_key, usage::AUTHORIZE_CHALLENGE, &CephXChallenge { server_challenge: c })?;
                return Ok(VerifyOutcome::Challenge { reply: Bytes::from(reply) });
            }
        }
    }

    let connection_secret = if connection_secret_len > 0 {
        let v: Vec<u8> = (0..connection_secret_len).map(|_| rand::random::<u8>()).collect();
        Bytes::from(v)
    } else {
        Bytes::new()
    };
    let reply_msg = CephXAuthorizeReply { nonce_plus_one: auth_msg.nonce.wrapping_add(1), connection_secret: connection_secret.clone() };
    let mut reply = BytesMut::new();
    encode_encrypt_value(&mut reply, &info.session_key, usage::AUTHORIZE_REPLY, &reply_msg)?;
    Ok(VerifyOutcome::Accepted(Box::new(VerifiedAuthorizer {
        name: info.ticket.name.clone(),
        global_id,
        caps: info.ticket.caps.clone(),
        session_key: info.session_key,
        reply: reply.freeze(),
        connection_secret,
    })))
}

/// The internal-consistency checks of `cephx_verify_authorizer`: sane nanoseconds,
/// `expires >= created`, and ticket creation within 300 s of the session key's creation.
fn check_ticket_timestamps(info: &CephXServiceTicketInfo) -> Result<(), AuthError> {
    let created = info.ticket.created;
    let expires = info.ticket.expires;
    let session_created = info.session_key.created();
    let secs = |u: UTime| f64::from(u.sec) + f64::from(u.nsec) * 1e-9;
    if created.nsec > 1_000_000_000
        || expires.nsec > 1_000_000_000
        || session_created.nsec > 1_000_000_000
        || expires < created
        || (secs(created) - secs(session_created)).abs() > 300.0
    {
        return Err(AuthError::Rejected("bad ticket timestamps".into()));
    }
    Ok(())
}

/// Test helper: the monitor side of the cephx exchange (`CephxServiceHandler::handle_request`
/// with an in-memory `KeyServer`). It lets the messenger and MonClient be tested without a
/// cluster. Not used by the daemon.
pub struct MonTicketIssuer {
    name: AuthName,
    key: CryptoKey,
    global_id: u64,
    server_challenge: u64,
    auth_secret: (u64, CryptoKey),
    services: BTreeMap<u32, RotatingSecrets>,
    caps: BTreeMap<u32, String>,
    auth_session_key: Option<CryptoKey>,
}

const TTL: u32 = 3600;

impl MonTicketIssuer {
    pub fn new(name: AuthName, key: CryptoKey, global_id: u64) -> Self {
        let now = crate::utime_now();
        let mut services = BTreeMap::new();
        for svc in [crate::ENTITY_MON, crate::ENTITY_OSD, crate::ENTITY_MDS, crate::ENTITY_MGR] {
            let mut r = RotatingSecrets::default();
            for i in 0..3u32 {
                r.max_ver += 1;
                r.secrets.insert(
                    r.max_ver,
                    ExpiringCryptoKey { key: CryptoKey::generate_aes(now), expiration: UTime { sec: now.sec + TTL * (i + 1), nsec: 0 } },
                );
            }
            services.insert(svc, r);
        }
        let mut caps = BTreeMap::new();
        caps.insert(crate::ENTITY_MDS, "allow".to_string());
        caps.insert(crate::ENTITY_MON, "allow profile mds".to_string());
        MonTicketIssuer {
            name,
            key,
            global_id,
            server_challenge: 0,
            auth_secret: (1, CryptoKey::generate_aes(now)),
            services,
            caps,
            auth_session_key: None,
        }
    }

    /// Set the cap string the issuer puts into tickets for `service`.
    pub fn set_caps(&mut self, service: u32, caps: &str) {
        self.caps.insert(service, caps.to_string());
    }

    /// The first reply of a session: `CephXServerChallenge`.
    pub fn server_challenge(&mut self) -> Bytes {
        self.server_challenge = rand::random::<u64>() | 1;
        denc::encode_to_bytes(&CephXChallenge { server_challenge: self.server_challenge }, 0)
    }

    /// The key store a daemon of type `service` would hold after fetching its rotating secrets.
    pub fn keystore(&self, service: u32) -> ServiceKeys {
        ServiceKeys { service_id: service, rotating: self.services[&service].clone() }
    }

    /// [`keystore`](Self::keystore) for the MDS service.
    pub fn mds_keystore(&self) -> ServiceKeys {
        self.keystore(crate::ENTITY_MDS)
    }

    fn session_info(&self, service: u32, name: &AuthName, global_id: u64) -> (u32, u64, CryptoKey, AuthTicket, CryptoKey) {
        let now = crate::utime_now();
        let (secret_id, secret) = if service == crate::ENTITY_AUTH {
            self.auth_secret.clone()
        } else {
            let r = &self.services[&service];
            let (id, k) = r.secrets.iter().nth(1).unwrap();
            (*id, k.key.clone())
        };
        let caps = match self.caps.get(&service) {
            Some(s) => denc::encode_to_bytes(s, 0),
            None => Bytes::new(),
        };
        let ticket = AuthTicket {
            name: name.clone(),
            global_id,
            auid: AUTH_UID_DEFAULT,
            created: now,
            expires: UTime { sec: now.sec + TTL, nsec: now.nsec },
            caps: AuthCapsInfo { allow_all: false, caps },
            flags: 0,
        };
        (service, secret_id, secret, ticket, CryptoKey::generate_aes(now))
    }

    fn build_reply(&self, principal: &CryptoKey, infos: Vec<(u32, u64, CryptoKey, AuthTicket, CryptoKey)>, out: &mut BytesMut) -> Result<(), AuthError> {
        out.put_u8(1);
        out.put_u32_le(infos.len() as u32);
        for (service, secret_id, secret, ticket, session_key) in infos {
            out.put_u32_le(service);
            out.put_u8(1);
            let a = CephXServiceTicket { session_key: session_key.clone(), validity: UTime { sec: TTL, nsec: 0 } };
            encode_encrypt_value(out, principal, usage::TICKET_SESSION_KEY, &a)?;
            let info = CephXServiceTicketInfo { ticket, session_key };
            let blob = encrypt_enc_bl_value(&secret, usage::TICKET_INFO, &info)?;
            let tb = CephXTicketBlob { secret_id, blob: Bytes::from(blob) };
            out.put_u8(0); // ticket not encrypted
            let tbb = denc::encode_to_bytes(&tb, 0);
            tbb.encode(out, 0);
        }
        Ok(())
    }

    /// Answer one request payload (the `MAuthReply` `result_bl`).
    pub fn handle_request(&mut self, req: Bytes) -> Result<Bytes, AuthError> {
        let mut p = req;
        let ty = u16::decode(&mut p)?;
        let mut out = BytesMut::new();
        match ty {
            crate::client::CEPHX_GET_AUTH_SESSION_KEY => {
                let a = CephXAuthenticate::decode(&mut p)?;
                let expect = crate::client::calc_client_server_challenge(&self.key, self.server_challenge, a.client_challenge)?;
                if expect != a.key {
                    return Err(AuthError::Rejected("bad challenge proof".into()));
                }
                CephXResponseHeader { request_type: ty, status: 0 }.encode(&mut out, 0);
                let info = self.session_info(crate::ENTITY_AUTH, &self.name.clone(), self.global_id);
                self.auth_session_key = Some(info.4.clone());
                let key = self.key.clone();
                self.build_reply(&key, vec![info], &mut out)?;
                Bytes::new().encode(&mut out, 0); // connection secret bl
                Bytes::new().encode(&mut out, 0); // extra tickets
            }
            crate::client::CEPHX_GET_PRINCIPAL_SESSION_KEY => {
                let ks = AuthKeys { secret: self.auth_secret.clone() };
                let v = match verify_authorizer(&ks, crate::ENTITY_AUTH, &p, None, 0)? {
                    VerifyOutcome::Accepted(v) => v,
                    VerifyOutcome::Challenge { .. } => unreachable!(),
                };
                // Skip the authorizer bytes: re-parse to find the ticket request at the end.
                let keys = u32::from_le_bytes(p[p.len() - 4..].try_into().unwrap());
                CephXResponseHeader { request_type: ty, status: 0 }.encode(&mut out, 0);
                let mut infos = Vec::new();
                let mut s = 1u32;
                while s != 0 && s <= keys {
                    if keys & s != 0 && s != crate::ENTITY_AUTH && self.services.contains_key(&s) {
                        infos.push(self.session_info(s, &v.name, v.global_id));
                    }
                    s <<= 1;
                }
                self.build_reply(&v.session_key, infos, &mut out)?;
            }
            crate::client::CEPHX_GET_ROTATING_KEY => {
                CephXResponseHeader { request_type: ty, status: 0 }.encode(&mut out, 0);
                let r = &self.services[&u32::from(self.name.ty.0)];
                encode_encrypt_value(&mut out, &self.key, usage::ROTATING_SECRET, r)?;
            }
            t => return Err(AuthError::Protocol(format!("unknown request {t:#x}"))),
        }
        Ok(out.freeze())
    }

    /// Issue a service ticket directly (as if `client_name` had fetched it), for tests of
    /// incoming connections: returns the authorizer payload of a fresh client handshake.
    pub fn client_for(&self, name: AuthName, key: CryptoKey, global_id: u64) -> Result<crate::CephxClient, AuthError> {
        let mut mon = MonTicketIssuer { name: name.clone(), key: key.clone(), global_id, ..self.clone_secrets() };
        let mut c = crate::CephxClient::new(name, key, crate::ENTITY_MON | crate::ENTITY_MDS | crate::ENTITY_OSD);
        c.handle_response(0, mon.server_challenge())?;
        c.set_global_id(global_id);
        let r = mon.handle_request(c.build_request()?)?;
        c.handle_response(0, r)?;
        let r = mon.handle_request(c.build_request()?)?;
        c.handle_response(0, r)?;
        Ok(c)
    }

    fn clone_secrets(&self) -> MonTicketIssuer {
        MonTicketIssuer {
            name: self.name.clone(),
            key: self.key.clone(),
            global_id: self.global_id,
            server_challenge: 0,
            auth_secret: self.auth_secret.clone(),
            services: self.services.clone(),
            caps: self.caps.clone(),
            auth_session_key: None,
        }
    }
}

struct AuthKeys {
    secret: (u64, CryptoKey),
}

impl KeyStore for AuthKeys {
    fn get_secret(&self, _name: &AuthName) -> Option<CryptoKey> {
        None
    }
    fn get_service_secret(&self, service_id: u32, secret_id: u64) -> Option<CryptoKey> {
        (service_id == crate::ENTITY_AUTH && secret_id == self.secret.0).then(|| self.secret.1.clone())
    }
}

/// Used by `MonTicketIssuer`; kept here so `encode_encrypt` stays referenced.
#[allow(dead_code)]
fn raw_envelope(key: &CryptoKey, payload: &[u8]) -> Result<Bytes, AuthError> {
    let mut b = BytesMut::new();
    encode_encrypt(&mut b, key, 0, payload)?;
    Ok(b.freeze())
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::EntityType;

    fn setup() -> (MonTicketIssuer, crate::CephxClient) {
        let mds = MonTicketIssuer::new(AuthName::new(EntityType::MDS, "a"), CryptoKey::generate_aes(crate::utime_now()), 1);
        let c = mds
            .client_for(AuthName::new(EntityType::CLIENT, "admin"), CryptoKey::generate_aes(crate::utime_now()), 77)
            .unwrap();
        (mds, c)
    }

    #[test]
    fn challenge_round_then_accept() {
        let (mds, c) = setup();
        let ks = mds.mds_keystore();
        let mut a = c.build_authorizer(crate::ENTITY_MDS).unwrap();
        let mut ch = None;
        let VerifyOutcome::Challenge { reply } = verify_authorizer(&ks, crate::ENTITY_MDS, &a.bl, Some(&mut ch), 0).unwrap() else {
            panic!()
        };
        a.add_challenge(&reply).unwrap();
        let VerifyOutcome::Accepted(v) = verify_authorizer(&ks, crate::ENTITY_MDS, &a.bl, Some(&mut ch), 0).unwrap() else {
            panic!()
        };
        assert_eq!(v.global_id, 77);
        assert_eq!(v.name.to_string(), "client.admin");
        assert_eq!(v.caps.cap_string().as_deref(), Some("allow"));
        a.verify_reply(&v.reply).unwrap();
    }

    #[test]
    fn rejects_wrong_challenge_and_foreign_service() {
        let (mds, c) = setup();
        let ks = mds.mds_keystore();
        let a = c.build_authorizer(crate::ENTITY_MDS).unwrap();
        // Without a challenge round (peer lacks CEPHX_V2): accepted immediately.
        assert!(matches!(verify_authorizer(&ks, crate::ENTITY_MDS, &a.bl, None, 0).unwrap(), VerifyOutcome::Accepted(_)));
        // An authorizer without the challenge answer just gets a new challenge (the messenger
        // then rejects it because a challenge was already sent).
        let mut ch = Some(5);
        assert!(matches!(
            verify_authorizer(&ks, crate::ENTITY_MDS, &a.bl, Some(&mut ch), 0).unwrap(),
            VerifyOutcome::Challenge { .. }
        ));
        // A wrong answer is refused.
        let mut ch = Some(5);
        let mut wrong = a.clone();
        let bogus = encrypt_enc_bl_value(&a.session_key, usage::AUTHORIZE_CHALLENGE, &CephXChallenge { server_challenge: 99 }).unwrap();
        wrong.add_challenge(&bogus).unwrap();
        assert!(verify_authorizer(&ks, crate::ENTITY_MDS, &wrong.bl, Some(&mut ch), 0).is_err());
        // An OSD ticket presented to the MDS is refused.
        let o = c.build_authorizer(crate::ENTITY_OSD).unwrap();
        assert!(verify_authorizer(&ks, crate::ENTITY_MDS, &o.bl, None, 0).is_err());
        // Garbage is refused.
        assert!(verify_authorizer(&ks, crate::ENTITY_MDS, &[1, 2, 3], None, 0).is_err());
    }
}
