//! cephx wire structures (`src/auth/Auth.h`, `src/auth/Auth.cc`,
//! `src/auth/cephx/CephxProtocol.h`). Each struct starts with a `u8 struct_v`; none uses the
//! `ENCODE_START` length framing.

use std::collections::BTreeMap;

use bytes::{BufMut, Bytes, BytesMut};
use denc::{Decode, DecodeError, Encode, UTime};

use crate::{AuthName, CryptoKey};

/// `CEPH_AUTH_UID_DEFAULT` (`(__u64)-1`): the only `auid` a valid ticket carries.
pub const AUTH_UID_DEFAULT: u64 = u64::MAX;

fn expect_v(buf: &mut Bytes, what: &'static str, min: u8) -> denc::Result<u8> {
    let v = u8::decode(buf)?;
    if v < min {
        return Err(DecodeError::Invalid {
            what,
            detail: format!("struct_v {v} < {min}"),
        });
    }
    Ok(v)
}

/// `AuthCapsInfo`: `u8 1`, `u8 allow_all`, `bufferlist caps` (the caps blob holds one encoded
/// string, the entity's cap text for the service).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthCapsInfo {
    pub allow_all: bool,
    pub caps: Bytes,
}

impl AuthCapsInfo {
    /// The cap string in the blob (`None` if the blob is empty or not exactly one string).
    pub fn cap_string(&self) -> Option<String> {
        if self.caps.is_empty() {
            return None;
        }
        denc::decode_exact::<String>(self.caps.clone(), "caps").ok()
    }
}

impl Encode for AuthCapsInfo {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u8(1);
        buf.put_u8(u8::from(self.allow_all));
        self.caps.encode(buf, f);
    }
}
impl Decode for AuthCapsInfo {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let _v = u8::decode(buf)?;
        let allow_all = u8::decode(buf)? != 0;
        let caps = Bytes::decode(buf)?;
        Ok(AuthCapsInfo { allow_all, caps })
    }
}

/// `AuthTicket`: `u8 2`, `EntityName name`, `u64 global_id`, `u64 auid` (v>=2, always
/// `CEPH_AUTH_UID_DEFAULT` when encoded), `utime created`, `utime expires`, `AuthCapsInfo`,
/// `u32 flags`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthTicket {
    pub name: AuthName,
    pub global_id: u64,
    pub auid: u64,
    pub created: UTime,
    pub expires: UTime,
    pub caps: AuthCapsInfo,
    pub flags: u32,
}

impl Encode for AuthTicket {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u8(2);
        self.name.encode(buf, f);
        buf.put_u64_le(self.global_id);
        buf.put_u64_le(AUTH_UID_DEFAULT);
        self.created.encode(buf, f);
        self.expires.encode(buf, f);
        self.caps.encode(buf, f);
        buf.put_u32_le(self.flags);
    }
}
impl Decode for AuthTicket {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let v = u8::decode(buf)?;
        let name = AuthName::decode(buf)?;
        let global_id = u64::decode(buf)?;
        let auid = if v >= 2 {
            u64::decode(buf)?
        } else {
            AUTH_UID_DEFAULT
        };
        let created = UTime::decode(buf)?;
        let expires = UTime::decode(buf)?;
        let caps = AuthCapsInfo::decode(buf)?;
        let flags = u32::decode(buf)?;
        Ok(AuthTicket {
            name,
            global_id,
            auid,
            created,
            expires,
            caps,
            flags,
        })
    }
}

/// `CephXTicketBlob`: `u8 1`, `u64 secret_id`, `bufferlist blob` (opaque to the principal: a
/// `CephXServiceTicketInfo` encrypted with the service secret `secret_id`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CephXTicketBlob {
    pub secret_id: u64,
    pub blob: Bytes,
}

impl Encode for CephXTicketBlob {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u8(1);
        buf.put_u64_le(self.secret_id);
        self.blob.encode(buf, f);
    }
}
impl Decode for CephXTicketBlob {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let _v = u8::decode(buf)?;
        Ok(CephXTicketBlob {
            secret_id: u64::decode(buf)?,
            blob: Bytes::decode(buf)?,
        })
    }
}

/// `CephXServiceTicketInfo`: `u8 1`, `AuthTicket`, session `CryptoKey`. Sealed with the
/// service secret under usage `TICKET_INFO`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CephXServiceTicketInfo {
    pub ticket: AuthTicket,
    pub session_key: CryptoKey,
}

impl Encode for CephXServiceTicketInfo {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u8(1);
        self.ticket.encode(buf, f);
        self.session_key.encode(buf, f);
    }
}
impl Decode for CephXServiceTicketInfo {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let _v = u8::decode(buf)?;
        Ok(CephXServiceTicketInfo {
            ticket: AuthTicket::decode(buf)?,
            session_key: CryptoKey::decode(buf)?,
        })
    }
}

/// `CephXServiceTicket` ("A"): `u8 1`, session `CryptoKey`, `utime validity`. Sealed with the
/// principal's secret (or the auth session key) under usage `TICKET_SESSION_KEY`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CephXServiceTicket {
    pub session_key: CryptoKey,
    pub validity: UTime,
}

impl Encode for CephXServiceTicket {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u8(1);
        self.session_key.encode(buf, f);
        self.validity.encode(buf, f);
    }
}
impl Decode for CephXServiceTicket {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let _v = u8::decode(buf)?;
        Ok(CephXServiceTicket {
            session_key: CryptoKey::decode(buf)?,
            validity: UTime::decode(buf)?,
        })
    }
}

/// `CephXAuthorize`: `u8 2`, `u64 nonce`, then (v>=2) `bool have_challenge`,
/// `u64 server_challenge_plus_one`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CephXAuthorize {
    pub nonce: u64,
    pub have_challenge: bool,
    pub server_challenge_plus_one: u64,
}

impl Encode for CephXAuthorize {
    fn encode(&self, buf: &mut BytesMut, _f: u64) {
        buf.put_u8(2);
        buf.put_u64_le(self.nonce);
        buf.put_u8(u8::from(self.have_challenge));
        buf.put_u64_le(self.server_challenge_plus_one);
    }
}
impl Decode for CephXAuthorize {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let v = u8::decode(buf)?;
        let nonce = u64::decode(buf)?;
        let (have_challenge, server_challenge_plus_one) = if v >= 2 {
            (bool::decode(buf)?, u64::decode(buf)?)
        } else {
            (false, 0)
        };
        Ok(CephXAuthorize {
            nonce,
            have_challenge,
            server_challenge_plus_one,
        })
    }
}

/// `CephXAuthorizeReply`: `u8 1` (or 2 with a connection secret), `u64 nonce_plus_one`,
/// `string connection_secret` (v2 only).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CephXAuthorizeReply {
    pub nonce_plus_one: u64,
    pub connection_secret: Bytes,
}

impl Encode for CephXAuthorizeReply {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        let v = if self.connection_secret.is_empty() {
            1
        } else {
            2
        };
        buf.put_u8(v);
        buf.put_u64_le(self.nonce_plus_one);
        if v >= 2 {
            self.connection_secret.encode(buf, f);
        }
    }
}
impl Decode for CephXAuthorizeReply {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let v = u8::decode(buf)?;
        let nonce_plus_one = u64::decode(buf)?;
        let connection_secret = if v >= 2 {
            Bytes::decode(buf)?
        } else {
            Bytes::new()
        };
        Ok(CephXAuthorizeReply {
            nonce_plus_one,
            connection_secret,
        })
    }
}

/// `CephXAuthorizeChallenge` and `CephXServerChallenge` share this layout: `u8 1`,
/// `u64 server_challenge`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CephXChallenge {
    pub server_challenge: u64,
}

impl Encode for CephXChallenge {
    fn encode(&self, buf: &mut BytesMut, _f: u64) {
        buf.put_u8(1);
        buf.put_u64_le(self.server_challenge);
    }
}
impl Decode for CephXChallenge {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let _v = u8::decode(buf)?;
        Ok(CephXChallenge {
            server_challenge: u64::decode(buf)?,
        })
    }
}

/// `CephXAuthenticate` (client answer to the server challenge): `u8 3`, `u64 client_challenge`,
/// `u64 key` (the challenge proof), `CephXTicketBlob old_ticket`, `u32 other_keys`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CephXAuthenticate {
    pub client_challenge: u64,
    pub key: u64,
    pub old_ticket: CephXTicketBlob,
    pub other_keys: u32,
}

impl Encode for CephXAuthenticate {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u8(3);
        buf.put_u64_le(self.client_challenge);
        buf.put_u64_le(self.key);
        self.old_ticket.encode(buf, f);
        buf.put_u32_le(self.other_keys);
    }
}
impl Decode for CephXAuthenticate {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let v = u8::decode(buf)?;
        let client_challenge = u64::decode(buf)?;
        let key = u64::decode(buf)?;
        let old_ticket = CephXTicketBlob::decode(buf)?;
        let other_keys = if v >= 2 { u32::decode(buf)? } else { 0 };
        Ok(CephXAuthenticate {
            client_challenge,
            key,
            old_ticket,
            other_keys,
        })
    }
}

/// `CephXRequestHeader`: `u16 request_type`.
pub fn encode_request_header(buf: &mut BytesMut, request_type: u16) {
    buf.put_u16_le(request_type);
}

/// `CephXResponseHeader`: `u16 request_type`, `i32 status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CephXResponseHeader {
    pub request_type: u16,
    pub status: i32,
}

impl Decode for CephXResponseHeader {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        Ok(CephXResponseHeader {
            request_type: u16::decode(buf)?,
            status: i32::decode(buf)?,
        })
    }
}
impl Encode for CephXResponseHeader {
    fn encode(&self, buf: &mut BytesMut, _f: u64) {
        buf.put_u16_le(self.request_type);
        buf.put_i32_le(self.status);
    }
}

/// `ExpiringCryptoKey`: `u8 1`, `CryptoKey`, `utime expiration`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiringCryptoKey {
    pub key: CryptoKey,
    pub expiration: UTime,
}

impl Encode for ExpiringCryptoKey {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u8(1);
        self.key.encode(buf, f);
        self.expiration.encode(buf, f);
    }
}
impl Decode for ExpiringCryptoKey {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let _v = expect_v(buf, "ExpiringCryptoKey", 1)?;
        Ok(ExpiringCryptoKey {
            key: CryptoKey::decode(buf)?,
            expiration: UTime::decode(buf)?,
        })
    }
}

/// `KEY_ROTATE_NUM`: a service holds previous, current and next secret.
pub const KEY_ROTATE_NUM: usize = 3;

/// `RotatingSecrets`: `u8 1`, `map<u64 secret_id, ExpiringCryptoKey>`, `u64 max_ver`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RotatingSecrets {
    pub secrets: BTreeMap<u64, ExpiringCryptoKey>,
    pub max_ver: u64,
}

impl RotatingSecrets {
    /// The second-oldest ("current") secret (`RotatingSecrets::current`).
    pub fn current(&self) -> Option<&ExpiringCryptoKey> {
        self.secrets.values().nth(1)
    }

    /// `need_new_secrets(now)`: fewer than three secrets, or the current one expired at or
    /// before `now`.
    pub fn need_new_secrets(&self, now: UTime) -> bool {
        self.secrets.len() < KEY_ROTATE_NUM || self.current().is_none_or(|c| c.expiration <= now)
    }

    /// Look up a secret by id.
    pub fn get(&self, secret_id: u64) -> Option<&CryptoKey> {
        self.secrets.get(&secret_id).map(|e| &e.key)
    }
}

impl Encode for RotatingSecrets {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u8(1);
        self.secrets.encode(buf, f);
        buf.put_u64_le(self.max_ver);
    }
}
impl Decode for RotatingSecrets {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let _v = u8::decode(buf)?;
        Ok(RotatingSecrets {
            secrets: BTreeMap::decode(buf)?,
            max_ver: u64::decode(buf)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::EntityType;

    fn h(b: &Bytes) -> String {
        hex::encode(b)
    }

    // Golden bytes from `ceph-dencoder type <T> select_test <n> encode export` (Ceph 20.2.0).
    #[test]
    fn golden_encodings() {
        // CephXAuthorize select_test 2: nonce 1234, have_challenge, plus_one 1234.
        let a = CephXAuthorize {
            nonce: 1234,
            have_challenge: true,
            server_challenge_plus_one: 1234,
        };
        assert_eq!(
            h(&denc::encode_to_bytes(&a, 0)),
            "02d20400000000000001d204000000000000"
        );
        // CephXTicketBlob select_test 1: secret_id 123, blob "this is a blob".
        let t = CephXTicketBlob {
            secret_id: 123,
            blob: Bytes::from_static(b"this is a blob"),
        };
        assert_eq!(
            h(&denc::encode_to_bytes(&t, 0)),
            "017b000000000000000e00000074686973206973206120626c6f62"
        );
        // CephXAuthenticate select_test 2.
        let au = CephXAuthenticate {
            client_challenge: 1,
            key: 2,
            old_ticket: CephXTicketBlob {
                secret_id: 3,
                blob: Bytes::from_static(b"this is a blob"),
            },
            other_keys: 4,
        };
        assert_eq!(
            h(&denc::encode_to_bytes(&au, 0)),
            "03010000000000000002000000000000000103000000000000000e00000074686973206973206120626c6f6204000000"
        );
        // CephXAuthorizeReply select_test 2: nonce_plus_one 123, secret "secret".
        let r = CephXAuthorizeReply {
            nonce_plus_one: 123,
            connection_secret: Bytes::from_static(b"secret"),
        };
        assert_eq!(
            h(&denc::encode_to_bytes(&r, 0)),
            "027b0000000000000006000000736563726574"
        );
        // AuthCapsInfo select_test 3: caps "foo" + "bar" appended raw.
        let c = AuthCapsInfo {
            allow_all: false,
            caps: Bytes::from_static(b"foobar"),
        };
        assert_eq!(h(&denc::encode_to_bytes(&c, 0)), "010006000000666f6f626172");
    }

    #[test]
    fn golden_decodes() {
        // AuthTicket select_test 2 (name type 0 with id "client.123", as set_id() leaves it).
        let b = Bytes::from(hex::decode("02000000000a000000636c69656e742e3132337b00000000000000ffffffffffffffff7b000000c801000082000000c8010000010006000000666f6f62617278563412").unwrap());
        let t = denc::decode_exact::<AuthTicket>(b.clone(), "AuthTicket").unwrap();
        assert_eq!(t.name.id, "client.123");
        assert_eq!(t.global_id, 123);
        assert_eq!(
            t.created,
            UTime {
                sec: 123,
                nsec: 456
            }
        );
        assert_eq!(
            t.expires,
            UTime {
                sec: 130,
                nsec: 456
            }
        );
        assert_eq!(t.flags, 0x12345678);
        assert_eq!(denc::encode_to_bytes(&t, 0), b);
        // CephXServiceTicketInfo select_test 2.
        let b = Bytes::from(hex::decode("01020000000000000000d204000000000000ffffffffffffffff7b000000c8010000f60000008f0300000100000000000000000001007b000000c8010000100031323334353637383930313233343536").unwrap());
        let i = denc::decode_exact::<CephXServiceTicketInfo>(b.clone(), "info").unwrap();
        assert_eq!(i.ticket.global_id, 1234);
        assert_eq!(i.session_key.secret(), b"1234567890123456");
        assert_eq!(denc::encode_to_bytes(&i, 0), b);
        // RotatingSecrets select_test 2: one empty secret with id 1, max_ver 1.
        let b = Bytes::from(hex::decode("010100000001000000000000000100000000000000000000000000000000000000000100000000000000").unwrap());
        let r = denc::decode_exact::<RotatingSecrets>(b.clone(), "rs").unwrap();
        assert_eq!(r.max_ver, 1);
        assert!(r.secrets[&1].key.is_empty());
        assert_eq!(denc::encode_to_bytes(&r, 0), b);
    }

    #[test]
    fn ticket_round_trip() {
        let t = AuthTicket {
            name: AuthName::new(EntityType::CLIENT, "123"),
            global_id: 123,
            auid: AUTH_UID_DEFAULT,
            created: UTime {
                sec: 123,
                nsec: 456,
            },
            expires: UTime {
                sec: 130,
                nsec: 456,
            },
            caps: AuthCapsInfo {
                allow_all: false,
                caps: Bytes::from_static(b"foobar"),
            },
            flags: 0x12345678,
        };
        let b = denc::encode_to_bytes(&t, 0);
        assert_eq!(denc::decode_exact::<AuthTicket>(b, "t").unwrap(), t);
    }
}
