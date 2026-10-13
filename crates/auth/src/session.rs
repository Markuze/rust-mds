//! msgr1 message signatures (`src/auth/cephx/CephxSessionHandler.cc`, `_calc_signature`).
//!
//! The signature goes into the `sig` field of the 21-byte `ceph_msg_footer` (with
//! `CEPH_MSG_FOOTER_SIGNED`) when `MSG_AUTH` was negotiated. Key = the cephx ticket session key
//! of the connection.
//!
//! - Peer with `CEPHX_V2`, AES key: encrypt the 32-byte block
//!   `{le32 header_crc, front_crc, front_len, middle_crc, middle_len, data_crc, data_len,
//!   low 32 bits of seq}` and XOR the first four little-endian 64-bit words of the ciphertext;
//!   AES256KRB5 key: XOR of the four words of HMAC-SHA256(secret, that block).
//! - Peer without `CEPHX_V2`: encrypt `{u8 1, le64 AUTH_ENC_MAGIC, le32 16, le32 header_crc,
//!   front_crc, middle_crc, data_crc}` and take the first 64-bit word.

use denc::features;

use crate::crypto::AUTH_ENC_MAGIC;
use crate::{AuthError, CryptoKey};

/// The message fields a signature covers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SigFields {
    pub header_crc: u32,
    pub front_crc: u32,
    pub front_len: u32,
    pub middle_crc: u32,
    pub middle_len: u32,
    pub data_crc: u32,
    pub data_len: u32,
    pub seq: u64,
}

/// `CephxSessionHandler`: signs and checks messages of one connection.
#[derive(Debug, Clone)]
pub struct SessionSigner {
    key: CryptoKey,
    features: u64,
}

impl SessionSigner {
    /// `None` if there is no session key (`get_auth_session_handler` returns no handler).
    pub fn new(key: CryptoKey, features: u64) -> Option<Self> {
        if key.is_empty() {
            None
        } else {
            Some(SessionSigner { key, features })
        }
    }

    /// Whether signatures are checked: only when `MSG_AUTH` was negotiated
    /// (`check_message_signature`).
    pub fn checks(&self) -> bool {
        self.features & features::MSG_AUTH != 0
    }

    /// `_calc_signature`.
    pub fn signature(&self, f: &SigFields) -> Result<u64, AuthError> {
        if !features::has(self.features, features::MASK_CEPHX_V2) {
            let mut b = Vec::with_capacity(29);
            b.push(1u8);
            b.extend_from_slice(&AUTH_ENC_MAGIC.to_le_bytes());
            b.extend_from_slice(&16u32.to_le_bytes());
            b.extend_from_slice(&f.header_crc.to_le_bytes());
            b.extend_from_slice(&f.front_crc.to_le_bytes());
            b.extend_from_slice(&f.middle_crc.to_le_bytes());
            b.extend_from_slice(&f.data_crc.to_le_bytes());
            let ct = self.key.encrypt(0, &b)?;
            return Ok(u64::from_le_bytes(ct[0..8].try_into().unwrap()));
        }
        let mut b = Vec::with_capacity(32);
        for v in [
            f.header_crc,
            f.front_crc,
            f.front_len,
            f.middle_crc,
            f.middle_len,
            f.data_crc,
            f.data_len,
            f.seq as u32,
        ] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        // AES: encrypt and fold the first 32 bytes; AES256KRB5: HMAC-SHA256 with the secret.
        let digest: Vec<u8> = if self.key.key_type() <= crate::crypto::CRYPTO_AES {
            self.key.encrypt(0, &b)?
        } else {
            self.key.hmac_sha256(&b).to_vec()
        };
        let mut sig = 0u64;
        for w in digest[..32].as_chunks::<8>().0 {
            sig ^= u64::from_le_bytes(*w);
        }
        Ok(sig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::UTime;

    #[test]
    fn v2_signature_matches_openssl() {
        // Reference: openssl enc -aes-128-cbc -K 303132...66 -iv <CEPH_AES_IV> over the 32-byte
        // block below; XOR of the first four LE words of the 48-byte output.
        let k = CryptoKey::new_aes(*b"0123456789abcdef", UTime::default());
        let s = SessionSigner::new(k, features::ALL).unwrap();
        let f = SigFields {
            header_crc: 0x11111111,
            front_crc: 0x22222222,
            front_len: 10,
            middle_crc: 0,
            middle_len: 0,
            data_crc: 0,
            data_len: 0,
            seq: 0x1_0000_0003,
        };
        assert_eq!(s.signature(&f).unwrap(), 0xebab_3d68_184c_6f79);
    }

    #[test]
    fn v2_signature_aes256krb5_is_hmac_sha256() {
        // Reference: Python hmac/hashlib over the same 32-byte block as above.
        let secret: Vec<u8> = (0..32u8).collect();
        let k =
            CryptoKey::new(crate::crypto::CRYPTO_AES256KRB5, UTime::default(), &secret).unwrap();
        let s = SessionSigner::new(k, features::ALL).unwrap();
        let f = SigFields {
            header_crc: 0x11111111,
            front_crc: 0x22222222,
            front_len: 10,
            seq: 3,
            ..Default::default()
        };
        assert_eq!(s.signature(&f).unwrap(), 0x5d0b_bbe1_5aa5_bd59);
    }

    #[test]
    fn legacy_signature_uses_first_word() {
        let k = CryptoKey::new_aes(*b"0123456789abcdef", UTime::default());
        let s = SessionSigner::new(k.clone(), features::MSG_AUTH).unwrap();
        let f = SigFields {
            header_crc: 1,
            front_crc: 2,
            middle_crc: 3,
            data_crc: 4,
            ..Default::default()
        };
        let mut b = vec![1u8];
        b.extend_from_slice(&AUTH_ENC_MAGIC.to_le_bytes());
        for v in [16u32, 1, 2, 3, 4] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        let ct = k.encrypt(0, &b).unwrap();
        assert_eq!(
            s.signature(&f).unwrap(),
            u64::from_le_bytes(ct[0..8].try_into().unwrap())
        );
        assert!(SessionSigner::new(CryptoKey::default(), 0).is_none());
    }
}
