//! cephx: keys, client handshake with the monitors, service tickets, authorizers, rotating
//! secrets, message signatures.
//!
//! - [`CryptoKey`], the encryption envelope and AES-128-CBC: [`crypto`].
//! - Keyring files: [`Keyring`].
//! - The principal side towards the monitors and outgoing authorizers: [`CephxClient`],
//!   [`ClientAuthorizer`].
//! - The service side (verifying incoming authorizers with the rotating secrets):
//!   [`service::verify_authorizer`].
//! - msgr1 message signatures: [`SessionSigner`].
//! - [`AuthContext`]: the state shared by a daemon's MonClient and messenger.
//!
//! Key types: 1 (AES-128-CBC, every Ceph 20.2 key; verified against real kernel and C++ traffic)
//! and 2 (AES256KRB5, Ceph after 20.2; verified against the RFC 8009 vectors only).

pub mod client;
mod context;
pub mod crypto;
mod keyring;
mod name;
pub mod service;
mod session;
pub mod types;

pub use client::{CephxClient, ClientAuthorizer, Progress};
pub use context::{AuthContext, AuthRequestOutcome, AuthorizerState};
pub use crypto::CryptoKey;
pub use keyring::{Keyring, KeyringEntry};
pub use name::AuthName;
pub use service::{KeyStore, ServiceKeys, VerifiedAuthorizer, VerifyOutcome};
pub use session::{SessionSigner, SigFields};
pub use types::{AuthCapsInfo, AuthTicket, RotatingSecrets};

use denc::{DecodeError, UTime};

/// `CEPH_AUTH_UNKNOWN`: the protocol of the first msgr1 `MAuth`.
pub const AUTH_UNKNOWN: u32 = 0;
/// `CEPH_AUTH_NONE`.
pub const AUTH_NONE: u32 = 1;
/// `CEPH_AUTH_CEPHX`.
pub const AUTH_CEPHX: u32 = 2;

/// `AUTH_MODE_AUTHORIZER` .. `AUTH_MODE_AUTHORIZER_MAX` (`src/auth/Auth.h`): the first byte of
/// an authorizer.
pub const AUTH_MODE_AUTHORIZER: u8 = 1;
pub const AUTH_MODE_AUTHORIZER_MAX: u8 = 9;

/// Entity type bits as `u32` masks (`want_keys`, ticket service ids).
pub const ENTITY_MON: u32 = 0x01;
pub const ENTITY_MDS: u32 = 0x02;
pub const ENTITY_OSD: u32 = 0x04;
pub const ENTITY_CLIENT: u32 = 0x08;
pub const ENTITY_MGR: u32 = 0x10;
pub const ENTITY_AUTH: u32 = 0x20;

/// cephx key usages (`CEPHX_KEY_USAGE_*`, `src/auth/cephx/CephxProtocol.h`). AES ignores them.
pub mod usage {
    pub const AUTH_CONNECTION_SECRET: u32 = 0x03;
    pub const TICKET_SESSION_KEY: u32 = 0x04;
    pub const TICKET_BLOB: u32 = 0x05;
    pub const AUTHORIZE: u32 = 0x10;
    pub const AUTHORIZE_CHALLENGE: u32 = 0x11;
    pub const AUTHORIZE_REPLY: u32 = 0x12;
    pub const ROTATING_SECRET: u32 = 0x20;
    pub const TICKET_INFO: u32 = 0x30;
}

/// Errors of this crate.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("crypto: {0}")]
    Crypto(String),
    #[error("bad magic in decrypted blob: {0:#x}")]
    BadMagic(u64),
    #[error("decode: {0}")]
    Decode(#[from] DecodeError),
    #[error("keyring: {0}")]
    Keyring(String),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("rejected: {0}")]
    Rejected(String),
    #[error("server returned error {0}")]
    Server(i32),
}

/// Current wall-clock time (`ceph_clock_now`).
pub fn utime_now() -> UTime {
    UTime::now()
}

/// `t + secs` (secs may be negative), saturating at zero.
pub fn utime_add_secs(t: UTime, secs: f64) -> UTime {
    let total = f64::from(t.sec) + f64::from(t.nsec) * 1e-9 + secs;
    if total <= 0.0 {
        return UTime::default();
    }
    let sec = total.floor();
    let nsec = ((total - sec) * 1e9).round().min(999_999_999.0);
    UTime { sec: sec as u32, nsec: nsec as u32 }
}
