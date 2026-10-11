//! `AuthContext`: the cephx state one daemon shares between its MonClient (which obtains
//! tickets and rotating secrets) and its messenger (which builds authorizers for outgoing
//! connections and verifies incoming ones). This is the role `MonClient` plays as
//! `AuthClient`/`AuthServer` of the C++ messenger (`src/mon/MonClient.cc`,
//! `get_auth_request`, `handle_auth_request`).

use std::sync::{Arc, Mutex, MutexGuard};

use bytes::Bytes;
use denc::{EntityType, UTime};

use crate::client::{CephxClient, ClientAuthorizer};
use crate::service::{ServiceKeys, VerifiedAuthorizer, VerifyOutcome, verify_authorizer};
use crate::types::RotatingSecrets;
use crate::{AUTH_CEPHX, AuthName, CryptoKey};

/// Per-connection state of an incoming authorizer exchange (`AuthConnectionMeta`).
#[derive(Debug, Clone, Default)]
pub struct AuthorizerState {
    /// The server challenge sent to the peer, if any.
    pub challenge: Option<u64>,
    /// Skip the challenge round (msgr1 peer without `CEPHX_V2`).
    pub skip_challenge: bool,
}

/// Result of [`AuthContext::handle_auth_request`].
#[derive(Debug)]
pub enum AuthRequestOutcome {
    /// Verified; reply with `reply` and the final connect tag.
    Accept(Box<VerifiedAuthorizer>),
    /// Reply with `CHALLENGE_AUTHORIZER` carrying this payload.
    Challenge(Bytes),
    /// Reply with `BADAUTHORIZER`.
    Reject(String),
}

/// Shared cephx state of one daemon.
#[derive(Debug)]
pub struct AuthContext {
    name: AuthName,
    key: CryptoKey,
    client: Mutex<CephxClient>,
    service: Mutex<ServiceKeys>,
}

impl AuthContext {
    /// `want_keys`: service types to fetch tickets for (`ENTITY_*` mask). The rotating secrets
    /// held are those of `name`'s own type.
    pub fn new(name: AuthName, key: CryptoKey, want_keys: u32) -> Arc<Self> {
        let client = CephxClient::new(name.clone(), key.clone(), want_keys);
        let service = ServiceKeys { service_id: u32::from(name.ty.0), rotating: RotatingSecrets::default() };
        Arc::new(AuthContext { name, key, client: Mutex::new(client), service: Mutex::new(service) })
    }

    pub fn name(&self) -> &AuthName {
        &self.name
    }

    pub fn key(&self) -> &CryptoKey {
        &self.key
    }

    /// The cephx client state (tickets, global id). Do not hold across `.await`.
    pub fn client(&self) -> MutexGuard<'_, CephxClient> {
        self.client.lock().unwrap()
    }

    pub fn global_id(&self) -> u64 {
        self.client().global_id()
    }

    /// Replace the rotating secrets (`RotatingKeyRing::set_secrets`).
    pub fn set_rotating(&self, r: RotatingSecrets) {
        self.service.lock().unwrap().rotating = r;
    }

    pub fn rotating(&self) -> RotatingSecrets {
        self.service.lock().unwrap().rotating.clone()
    }

    /// Wipe rotating secrets and invalidate all tickets (MonMap `auth_epoch` change).
    pub fn wipe_secrets_and_tickets(&self) {
        self.service.lock().unwrap().rotating = RotatingSecrets::default();
        self.client().invalidate_all_tickets();
    }

    /// `need_new_secrets(cutoff)` of the held rotating secrets.
    pub fn need_rotating(&self, cutoff: UTime) -> bool {
        self.service.lock().unwrap().rotating.need_new_secrets(cutoff)
    }

    /// Authorizer for an outgoing connection to a service (`MonClient::get_auth_request`).
    /// `None` when there is no valid ticket for that service type.
    pub fn build_authorizer(&self, peer_type: EntityType) -> Option<ClientAuthorizer> {
        self.client().build_authorizer(u32::from(peer_type.0))
    }

    /// Verify an incoming authorizer (`MonClient::handle_auth_request`). `more` is true when
    /// a challenge was already sent on this connection.
    pub fn handle_auth_request(&self, protocol: u32, payload: &[u8], more: bool, state: &mut AuthorizerState) -> AuthRequestOutcome {
        if payload.is_empty() {
            return AuthRequestOutcome::Reject("no authorizer".into());
        }
        let mode = payload[0];
        if !(crate::AUTH_MODE_AUTHORIZER..=crate::AUTH_MODE_AUTHORIZER_MAX).contains(&mode) {
            return AuthRequestOutcome::Reject(format!("bad auth mode {mode}"));
        }
        if protocol != AUTH_CEPHX {
            return AuthRequestOutcome::Reject(format!("unsupported auth method {protocol}"));
        }
        let was_challenge = state.challenge.is_some();
        let keys = self.service.lock().unwrap().clone();
        let ch = if state.skip_challenge { None } else { Some(&mut state.challenge) };
        match verify_authorizer(&keys, keys.service_id, payload, ch, 0) {
            Ok(VerifyOutcome::Accepted(v)) => AuthRequestOutcome::Accept(v),
            Ok(VerifyOutcome::Challenge { reply }) if !more && !was_challenge => AuthRequestOutcome::Challenge(reply),
            Ok(VerifyOutcome::Challenge { .. }) => {
                state.challenge = None;
                AuthRequestOutcome::Reject("authorizer did not answer the challenge".into())
            }
            Err(e) => {
                state.challenge = None;
                AuthRequestOutcome::Reject(e.to_string())
            }
        }
    }
}
