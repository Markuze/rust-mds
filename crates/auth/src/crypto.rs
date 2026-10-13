//! `CryptoKey` and the cephx encryption envelope.
//!
//! Follows `src/auth/Crypto.cc` (`CryptoKey::encode/decode`, `CryptoAESKeyHandler`) and the
//! `encode_encrypt*`/`decode_decrypt*` templates of `src/auth/cephx/CephxProtocol.h`.
//!
//! Wire format of a key (`CryptoKey::encode`): `u16 type`, `utime_t created` (`u32 sec`,
//! `u32 nsec`), `u16 len`, `len` bytes of secret. The keyring's `key = ...` value is the
//! base64 of exactly these bytes.
//!
//! Key type 1 (`CEPH_CRYPTO_AES`) is AES-128-CBC with the fixed IV `"cephsageyudagreg"`
//! (`CEPH_AES_IV`) and PKCS#7-style padding: there is always at least one byte of padding, so
//! the ciphertext of `n` bytes is `16 + n - n % 16` bytes long. Decryption strips
//! `min(last byte, 16)` bytes without checking the other padding bytes, as the C++ handler does
//! (`CryptoAESKeyHandler::decrypt_ext`). The key usage argument is ignored for AES.
//!
//! Key type 2 (`CEPH_CRYPTO_AES256KRB5`, Ceph after 20.2, `CryptoAES256KRB5KeyHandler`) is
//! RFC 8009 `aes256-cts-hmac-sha384-192`: per key usage, `Ke` (32 bytes) and `Ki` (24 bytes)
//! are derived with `HMAC-SHA384(secret, 00000001 | usage_be32 | 0xAA or 0x55 | 00 | bits_be32)`;
//! a message is `AES-256-CBC-CS3(Ke, IV = 0, confounder(16) | plaintext)` followed by the first
//! 24 bytes of `HMAC-SHA384(Ki, 0^16 | ciphertext)`.
//!
//! [`CryptoKey::hmac_sha256`] (HMAC-SHA256 keyed with the raw secret) serves the AES256KRB5
//! variants of the cephx challenge proof and of msgr1 message signatures.
//!
//! The encryption envelope (`encode_encrypt_enc_bl`) is the ciphertext of
//! `{u8 1, u64 AUTH_ENC_MAGIC, payload}`; `encode_encrypt` prefixes it with a `u32` length.

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::{Aes128, Aes256};
use base64::Engine;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use denc::{Decode, DecodeError, Encode, UTime};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha384};

use crate::AuthError;

/// `CEPH_CRYPTO_NONE`.
pub const CRYPTO_NONE: u16 = 0;
/// `CEPH_CRYPTO_AES` (AES-128-CBC).
pub const CRYPTO_AES: u16 = 1;
/// `CEPH_CRYPTO_AES256KRB5` (AES256-CTS-HMAC384-192, Ceph after 20.2).
pub const CRYPTO_AES256KRB5: u16 = 2;

/// `AUTH_ENC_MAGIC` (`src/auth/cephx/CephxProtocol.h`).
pub const AUTH_ENC_MAGIC: u64 = 0xff00_9cad_8826_aa55;

/// `CEPH_AES_IV` (`src/include/ceph_fs.h`).
pub const CEPH_AES_IV: &[u8; 16] = b"cephsageyudagreg";

const AES_BLOCK: usize = 16;
const AES_KEY_LEN: usize = 16;
const KRB5_KEY_LEN: usize = 32;
const KRB5_HASH_LEN: usize = 24;

/// A cephx key (`CryptoKey`): type, creation time and secret.
#[derive(Clone)]
pub struct CryptoKey {
    ty: u16,
    created: UTime,
    secret: Bytes,
    aes: Option<Aes128>,
}

impl std::fmt::Debug for CryptoKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the secret.
        f.debug_struct("CryptoKey")
            .field("type", &self.ty)
            .field("created", &self.created)
            .field("len", &self.secret.len())
            .finish()
    }
}

impl PartialEq for CryptoKey {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.created == other.created && self.secret == other.secret
    }
}
impl Eq for CryptoKey {}

impl Default for CryptoKey {
    fn default() -> Self {
        CryptoKey {
            ty: CRYPTO_NONE,
            created: UTime::default(),
            secret: Bytes::new(),
            aes: None,
        }
    }
}

impl CryptoKey {
    /// Build a key, validating the secret as `CryptoKey::_set_secret` does: an empty secret
    /// gives an empty key of any type; AES needs at least 16 bytes (only the first 16 are used).
    pub fn new(ty: u16, created: UTime, secret: &[u8]) -> Result<Self, AuthError> {
        if secret.is_empty() {
            return Ok(CryptoKey {
                ty,
                created,
                secret: Bytes::new(),
                aes: None,
            });
        }
        match ty {
            CRYPTO_AES => {
                if secret.len() < AES_KEY_LEN {
                    return Err(AuthError::Crypto(format!(
                        "AES secret too short: {} bytes",
                        secret.len()
                    )));
                }
                let aes = Aes128::new(GenericArray::from_slice(&secret[..AES_KEY_LEN]));
                Ok(CryptoKey {
                    ty,
                    created,
                    secret: Bytes::copy_from_slice(secret),
                    aes: Some(aes),
                })
            }
            CRYPTO_NONE => Ok(CryptoKey {
                ty,
                created,
                secret: Bytes::copy_from_slice(secret),
                aes: None,
            }),
            CRYPTO_AES256KRB5 => {
                if secret.len() < KRB5_KEY_LEN {
                    return Err(AuthError::Crypto(format!(
                        "AES256KRB5 secret too short: {} bytes",
                        secret.len()
                    )));
                }
                Ok(CryptoKey {
                    ty,
                    created,
                    secret: Bytes::copy_from_slice(secret),
                    aes: None,
                })
            }
            t => Err(AuthError::Crypto(format!("unknown key type {t}"))),
        }
    }

    /// An AES key from a 16-byte secret.
    pub fn new_aes(secret: [u8; 16], created: UTime) -> Self {
        Self::new(CRYPTO_AES, created, &secret).expect("16-byte AES secret is valid")
    }

    /// A fresh random AES key (`CryptoKey::create`).
    pub fn generate_aes(created: UTime) -> Self {
        Self::new_aes(rand::random::<[u8; 16]>(), created)
    }

    /// A fresh random AES256KRB5 key.
    pub fn generate_aes256krb5(created: UTime) -> Self {
        Self::new(CRYPTO_AES256KRB5, created, &rand::random::<[u8; 32]>())
            .expect("32-byte secret is valid")
    }

    pub fn key_type(&self) -> u16 {
        self.ty
    }

    pub fn created(&self) -> UTime {
        self.created
    }

    pub fn secret(&self) -> &[u8] {
        &self.secret
    }

    /// True if there is no secret (`CryptoKey::empty`).
    pub fn is_empty(&self) -> bool {
        self.secret.is_empty()
    }

    fn cipher(&self) -> Result<&Aes128, AuthError> {
        match (&self.aes, self.ty) {
            (Some(a), CRYPTO_AES) => Ok(a),
            _ if self.is_empty() => Err(AuthError::Crypto("empty key".into())),
            _ => Err(AuthError::Crypto(format!(
                "encryption with key type {} is not supported",
                self.ty
            ))),
        }
    }

    /// `CryptoKey::encrypt_ext`. `usage` only matters for AES256KRB5.
    pub fn encrypt(&self, usage: u32, plaintext: &[u8]) -> Result<Vec<u8>, AuthError> {
        if self.ty == CRYPTO_AES256KRB5 && !self.is_empty() {
            return Ok(self.krb5_encrypt(usage, plaintext, &rand::random::<[u8; 16]>()));
        }
        let aes = self.cipher()?;
        let pad = AES_BLOCK - plaintext.len() % AES_BLOCK; // 1..=16, always present
        let mut out = Vec::with_capacity(plaintext.len() + pad);
        out.extend_from_slice(plaintext);
        out.resize(plaintext.len() + pad, pad as u8);
        let mut iv = *CEPH_AES_IV;
        for chunk in out.as_chunks_mut::<AES_BLOCK>().0 {
            for (b, v) in chunk.iter_mut().zip(iv.iter()) {
                *b ^= v;
            }
            aes.encrypt_block(GenericArray::from_mut_slice(chunk));
            iv = *chunk;
        }
        Ok(out)
    }

    /// `CryptoKey::decrypt_ext`. `usage` only matters for AES256KRB5.
    pub fn decrypt(&self, usage: u32, ciphertext: &[u8]) -> Result<Vec<u8>, AuthError> {
        if self.ty == CRYPTO_AES256KRB5 && !self.is_empty() {
            return self.krb5_decrypt(usage, ciphertext);
        }
        let aes = self.cipher()?;
        if ciphertext.len() < AES_BLOCK || !ciphertext.len().is_multiple_of(AES_BLOCK) {
            return Err(AuthError::Crypto(format!(
                "ciphertext length {} not a multiple of 16",
                ciphertext.len()
            )));
        }
        let mut out = ciphertext.to_vec();
        let mut iv = *CEPH_AES_IV;
        for chunk in out.as_chunks_mut::<AES_BLOCK>().0 {
            let next_iv = *chunk;
            aes.decrypt_block(GenericArray::from_mut_slice(chunk));
            for (b, v) in chunk.iter_mut().zip(iv.iter()) {
                *b ^= v;
            }
            iv = next_iv;
        }
        // As in C++: no check of the padding bytes, only strip min(last, 16).
        let pad = usize::from(*out.last().unwrap()).min(AES_BLOCK);
        out.truncate(out.len() - pad);
        Ok(out)
    }

    /// `CryptoKeyHandler::hmac_sha256`: HMAC-SHA256 keyed with the raw secret.
    pub fn hmac_sha256(&self, data: &[u8]) -> [u8; 32] {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.secret)
            .expect("HMAC accepts any key length");
        mac.update(data);
        mac.finalize().into_bytes().into()
    }

    /// RFC 8009 encryption with a given confounder (`encrypt_ext` with `confounder`).
    pub fn krb5_encrypt(&self, usage: u32, plaintext: &[u8], confounder: &[u8; 16]) -> Vec<u8> {
        let ke = krb5_kdf(&self.secret, usage, 0xAA, KRB5_KEY_LEN);
        let ki = krb5_kdf(&self.secret, usage, 0x55, KRB5_HASH_LEN);
        let mut p = Vec::with_capacity(16 + plaintext.len());
        p.extend_from_slice(confounder);
        p.extend_from_slice(plaintext);
        let mut c = cbc_cs3_encrypt(&Aes256::new(GenericArray::from_slice(&ke)), &p);
        let h = hmac_sha384(&ki, &[&[0u8; 16], &c]);
        c.extend_from_slice(&h[..KRB5_HASH_LEN]);
        c
    }

    fn krb5_decrypt(&self, usage: u32, ciphertext: &[u8]) -> Result<Vec<u8>, AuthError> {
        if ciphertext.len() < AES_BLOCK + KRB5_HASH_LEN {
            return Err(AuthError::Crypto(format!(
                "AES256KRB5 ciphertext too short: {}",
                ciphertext.len()
            )));
        }
        let (c, h) = ciphertext.split_at(ciphertext.len() - KRB5_HASH_LEN);
        let ki = krb5_kdf(&self.secret, usage, 0x55, KRB5_HASH_LEN);
        let mut mac =
            <Hmac<Sha384> as Mac>::new_from_slice(&ki).expect("HMAC accepts any key length");
        mac.update(&[0u8; 16]);
        mac.update(c);
        mac.verify_truncated_left(h)
            .map_err(|_| AuthError::Crypto("AES256KRB5 integrity check failed".into()))?;
        let ke = krb5_kdf(&self.secret, usage, 0xAA, KRB5_KEY_LEN);
        let mut p = cbc_cs3_decrypt(&Aes256::new(GenericArray::from_slice(&ke)), c)?;
        p.drain(..AES_BLOCK); // the confounder
        Ok(p)
    }

    /// Decode the keyring/base64 form (`CryptoKey::decode_base64`).
    pub fn from_base64(s: &str) -> Result<Self, AuthError> {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(s.trim())
            .map_err(|e| AuthError::Crypto(format!("bad base64 key: {e}")))?;
        let mut b = Bytes::from(raw);
        let key = CryptoKey::decode(&mut b)?;
        Ok(key)
    }

    /// The keyring/base64 form (`CryptoKey::encode_base64`).
    pub fn to_base64(&self) -> String {
        let b = denc::encode_to_bytes(self, 0);
        base64::engine::general_purpose::STANDARD.encode(&b)
    }
}

impl Encode for CryptoKey {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u16_le(self.ty);
        self.created.encode(buf, f);
        buf.put_u16_le(self.secret.len() as u16);
        buf.put_slice(&self.secret);
    }
}

impl Decode for CryptoKey {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let ty = u16::decode(buf)?;
        let created = UTime::decode(buf)?;
        let len = u16::decode(buf)? as usize;
        let secret = denc::take(buf, len, "CryptoKey secret")?;
        CryptoKey::new(ty, created, &secret).map_err(|e| DecodeError::Invalid {
            what: "CryptoKey",
            detail: e.to_string(),
        })
    }
}

fn hmac_sha384(key: &[u8], parts: &[&[u8]]) -> [u8; 48] {
    let mut mac = <Hmac<Sha384> as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
    for p in parts {
        mac.update(p);
    }
    mac.finalize().into_bytes().into()
}

/// RFC 8009 KDF-HMAC-SHA2 (`calc_kx`): the first `k` bytes of
/// `HMAC-SHA384(secret, 00000001 | usage_be32 | ty | 00 | (8k)_be32)`.
pub fn krb5_kdf(secret: &[u8], usage: u32, ty: u8, k: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(14);
    data.extend_from_slice(&[0, 0, 0, 1]);
    data.extend_from_slice(&usage.to_be_bytes());
    data.push(ty);
    data.push(0);
    data.extend_from_slice(&((k * 8) as u32).to_be_bytes());
    hmac_sha384(secret, &[&data])[..k].to_vec()
}

/// AES-256-CBC with ciphertext stealing, variant CS3 (Kerberos: the last two blocks are always
/// swapped), zero IV. Input of at least one block.
fn cbc_cs3_encrypt(aes: &Aes256, p: &[u8]) -> Vec<u8> {
    let n = p.len();
    assert!(n >= AES_BLOCK, "CS3 needs at least one block");
    let m = n.div_ceil(AES_BLOCK);
    let mut prev = [0u8; AES_BLOCK];
    let mut blocks = Vec::with_capacity(m);
    for i in 0..m {
        let mut b = [0u8; AES_BLOCK];
        let chunk = &p[i * AES_BLOCK..((i + 1) * AES_BLOCK).min(n)];
        b[..chunk.len()].copy_from_slice(chunk);
        for (x, y) in b.iter_mut().zip(prev.iter()) {
            *x ^= y;
        }
        aes.encrypt_block(GenericArray::from_mut_slice(&mut b));
        blocks.push(b);
        prev = b;
    }
    if m == 1 {
        return blocks[0].to_vec();
    }
    let d = n - AES_BLOCK * (m - 1);
    let mut out = Vec::with_capacity(n);
    for b in &blocks[..m - 2] {
        out.extend_from_slice(b);
    }
    out.extend_from_slice(&blocks[m - 1]);
    out.extend_from_slice(&blocks[m - 2][..d]);
    out
}

fn cbc_cs3_decrypt(aes: &Aes256, c: &[u8]) -> Result<Vec<u8>, AuthError> {
    let n = c.len();
    if n < AES_BLOCK {
        return Err(AuthError::Crypto(
            "CS3 ciphertext shorter than a block".into(),
        ));
    }
    let dec = |b: &[u8]| -> [u8; AES_BLOCK] {
        let mut x = [0u8; AES_BLOCK];
        x.copy_from_slice(b);
        aes.decrypt_block(GenericArray::from_mut_slice(&mut x));
        x
    };
    if n == AES_BLOCK {
        return Ok(dec(c).to_vec());
    }
    let m = n.div_ceil(AES_BLOCK);
    let d = n - AES_BLOCK * (m - 1);
    // On the wire: C_1 .. C_{m-2}, C_m, first d bytes of C_{m-1}.
    let cm = &c[AES_BLOCK * (m - 2)..AES_BLOCK * (m - 1)];
    let tail = &c[AES_BLOCK * (m - 1)..];
    let z = dec(cm); // = P_m (zero padded) xor C_{m-1}
    let mut cm1 = [0u8; AES_BLOCK];
    cm1[..d].copy_from_slice(tail);
    cm1[d..].copy_from_slice(&z[d..]);
    let mut out = Vec::with_capacity(n);
    let mut prev = [0u8; AES_BLOCK];
    for i in 0..m - 2 {
        let ci = &c[AES_BLOCK * i..AES_BLOCK * (i + 1)];
        let mut p = dec(ci);
        for (x, y) in p.iter_mut().zip(prev.iter()) {
            *x ^= y;
        }
        out.extend_from_slice(&p);
        prev.copy_from_slice(ci);
    }
    let mut p = dec(&cm1);
    for (x, y) in p.iter_mut().zip(prev.iter()) {
        *x ^= y;
    }
    out.extend_from_slice(&p);
    for i in 0..d {
        out.push(z[i] ^ cm1[i]);
    }
    Ok(out)
}

/// `encode_encrypt_enc_bl`: the ciphertext of `{u8 1, u64 AUTH_ENC_MAGIC, payload}`.
pub fn encrypt_enc_bl(key: &CryptoKey, usage: u32, payload: &[u8]) -> Result<Vec<u8>, AuthError> {
    let mut plain = Vec::with_capacity(9 + payload.len());
    plain.push(1u8);
    plain.extend_from_slice(&AUTH_ENC_MAGIC.to_le_bytes());
    plain.extend_from_slice(payload);
    key.encrypt(usage, &plain)
}

/// `encode_encrypt`: `u32` length + [`encrypt_enc_bl`].
pub fn encode_encrypt(
    buf: &mut BytesMut,
    key: &CryptoKey,
    usage: u32,
    payload: &[u8],
) -> Result<(), AuthError> {
    let ct = encrypt_enc_bl(key, usage, payload)?;
    buf.put_u32_le(ct.len() as u32);
    buf.put_slice(&ct);
    Ok(())
}

/// `decode_decrypt_enc_bl`: decrypt, check `struct_v` and the magic, return the payload.
pub fn decrypt_enc_bl(key: &CryptoKey, usage: u32, ciphertext: &[u8]) -> Result<Bytes, AuthError> {
    let plain = key.decrypt(usage, ciphertext)?;
    let mut b = Bytes::from(plain);
    if b.remaining() < 9 {
        return Err(AuthError::Crypto("decrypted blob too short".into()));
    }
    let _struct_v = b.get_u8();
    let magic = b.get_u64_le();
    if magic != AUTH_ENC_MAGIC {
        return Err(AuthError::BadMagic(magic));
    }
    Ok(b)
}

/// `decode_decrypt`: read a `u32`-length ciphertext from `buf` and [`decrypt_enc_bl`] it.
pub fn decode_decrypt(buf: &mut Bytes, key: &CryptoKey, usage: u32) -> Result<Bytes, AuthError> {
    let ct = Bytes::decode(buf)?;
    decrypt_enc_bl(key, usage, &ct)
}

/// `encode_encrypt` of an encodable value.
pub fn encode_encrypt_value<T: Encode>(
    buf: &mut BytesMut,
    key: &CryptoKey,
    usage: u32,
    value: &T,
) -> Result<(), AuthError> {
    let payload = denc::encode_to_bytes(value, 0);
    encode_encrypt(buf, key, usage, &payload)
}

/// `encode_encrypt_enc_bl` of an encodable value.
pub fn encrypt_enc_bl_value<T: Encode>(
    key: &CryptoKey,
    usage: u32,
    value: &T,
) -> Result<Vec<u8>, AuthError> {
    let payload = denc::encode_to_bytes(value, 0);
    encrypt_enc_bl(key, usage, &payload)
}

/// `decode_decrypt` of a decodable value (trailing bytes are ignored, as in C++).
pub fn decode_decrypt_value<T: Decode>(
    buf: &mut Bytes,
    key: &CryptoKey,
    usage: u32,
) -> Result<T, AuthError> {
    let mut payload = decode_decrypt(buf, key, usage)?;
    Ok(T::decode(&mut payload)?)
}

/// `decode_decrypt_enc_bl` of a decodable value.
pub fn decrypt_enc_bl_value<T: Decode>(
    key: &CryptoKey,
    usage: u32,
    ciphertext: &[u8],
) -> Result<T, AuthError> {
    let mut payload = decrypt_enc_bl(key, usage, ciphertext)?;
    Ok(T::decode(&mut payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 16] = *b"0123456789abcdef";

    // Reference ciphertexts from `openssl enc -aes-128-cbc -K 30313233343536373839616263646566
    // -iv 63657068736167657975646167726567` (PKCS#7 padding, the same as Ceph's AES handler).
    #[test]
    fn aes_cbc_matches_openssl() {
        let k = CryptoKey::new_aes(KEY, UTime::default());
        // 0 bytes -> one block of padding.
        assert_eq!(
            hex::encode(k.encrypt(0, b"").unwrap()),
            "5457e8df2aea322e35357a19309cd610"
        );
        // 16 bytes -> two blocks (a full padding block is added).
        assert_eq!(
            hex::encode(k.encrypt(0, b"YELLOW SUBMARINE").unwrap()),
            "8b3aa75e5e4e008bb5403028033685cbdb96c740ed02c826998fdeea8355e6c0"
        );
        // 23 bytes.
        assert_eq!(
            hex::encode(k.encrypt(0, b"the quick brown fox jum").unwrap()),
            "1cf6a8def65119f2f83fc55ca6c62347905e7121090bb21ee6dbd4740e44eaed"
        );
    }

    #[test]
    fn aes_round_trip_all_lengths() {
        let k = CryptoKey::generate_aes(UTime::now());
        for n in 0..70 {
            let p: Vec<u8> = (0..n as u8).collect();
            let c = k.encrypt(0, &p).unwrap();
            assert_eq!(c.len(), 16 + n - n % 16);
            assert_eq!(k.decrypt(0, &c).unwrap(), p);
        }
        assert!(k.decrypt(0, &[0u8; 15]).is_err());
    }

    #[test]
    fn key_encoding_and_base64() {
        let k = CryptoKey::new_aes(
            KEY,
            UTime {
                sec: 0x01020304,
                nsec: 5,
            },
        );
        let b = denc::encode_to_bytes(&k, 0);
        assert_eq!(
            hex::encode(&b),
            "01000403020105000000100030313233343536373839616263646566"
        );
        let back = CryptoKey::from_base64(&k.to_base64()).unwrap();
        assert_eq!(back, k);
    }

    // RFC 8009 appendix A (aes256-cts-hmac-sha384-192), cross-checked with an independent
    // Python implementation (`cryptography`): base key, key usage 2.
    const RFC8009_BASE: &str = "6d404d37faf79f9df0d33568d320669800eb4836472ea8a026d16b7182460c52";

    #[test]
    fn aes256krb5_key_derivation_rfc8009() {
        let base = hex::decode(RFC8009_BASE).unwrap();
        assert_eq!(
            hex::encode(krb5_kdf(&base, 2, 0x99, 24)),
            "ef5718be86cc84963d8bbb5031e9f5c4ba41f28faf69e73d"
        );
        assert_eq!(
            hex::encode(krb5_kdf(&base, 2, 0xAA, 32)),
            "56ab22bee63d82d7bc5227f6773f8ea7a5eb1c825160c38312980c442e5c7e49"
        );
        assert_eq!(
            hex::encode(krb5_kdf(&base, 2, 0x55, 24)),
            "69b16514e3cd8e56b82010d5c73012b622c4d00ffc23ed1f"
        );
    }

    #[test]
    fn aes256krb5_encryption_rfc8009() {
        let k = CryptoKey::new(
            CRYPTO_AES256KRB5,
            UTime::default(),
            &hex::decode(RFC8009_BASE).unwrap(),
        )
        .unwrap();
        let cases = [
            (
                "f764e9fa15c276478b2c7d0c4e5f58e4",
                "",
                "41f53fa5bfe7026d91faf9be959195a058707273a96a40f0a01960621ac612748b9bbfbe7eb4ce3c",
            ),
            (
                "b80d3251c1f6471494256ffe712d0b9a",
                "000102030405",
                "4ed7b37c2bcac8f74f23c1cf07e62bc7b75fb3f637b9f559c7f664f69eab7b6092237526ea0d1f61cb20d69d10f2",
            ),
            (
                "53bf8a0d105265d4e276428624ce5e63",
                "000102030405060708090a0b0c0d0e0f",
                "bc47ffec7998eb91e8115cf8d19dac4bbbe2e163e87dd37f49beca92027764f68cf51f14d798c2273f35df574d1f932e40c4ff255b36a266",
            ),
            (
                "763e65367e864f02f55153c7e3b58af1",
                "000102030405060708090a0b0c0d0e0f1011121314",
                "40013e2df58e8751957d2878bcd2d6fe101ccfd556cb1eae79db3c3ee86429f2b2a602ac86fef6ecb647d6295fae077a1feb517508d2c16b4192e01f62",
            ),
        ];
        for (conf, pt, ct) in cases {
            let conf: [u8; 16] = hex::decode(conf).unwrap().try_into().unwrap();
            let pt = hex::decode(pt).unwrap();
            let out = k.krb5_encrypt(2, &pt, &conf);
            assert_eq!(hex::encode(&out), ct);
            assert_eq!(k.decrypt(2, &out).unwrap(), pt);
            // A different usage derives other keys: the integrity check fails.
            assert!(k.decrypt(3, &out).is_err());
            let mut bad = out.clone();
            bad[0] ^= 1;
            assert!(k.decrypt(2, &bad).is_err());
        }
        // Random confounders round-trip for all lengths.
        let k = CryptoKey::generate_aes256krb5(UTime::now());
        for n in 0..70 {
            let p: Vec<u8> = (0..n as u8).collect();
            let c = k.encrypt(5, &p).unwrap();
            assert_eq!(c.len(), 16 + n + 24);
            assert_eq!(k.decrypt(5, &c).unwrap(), p);
        }
        let b = CryptoKey::from_base64(&k.to_base64()).unwrap();
        assert_eq!(b, k);
    }

    #[test]
    fn envelope_round_trip_and_magic() {
        let k = CryptoKey::generate_aes(UTime::now());
        let mut buf = BytesMut::new();
        encode_encrypt(&mut buf, &k, 0, b"hello").unwrap();
        let mut b = buf.freeze();
        assert_eq!(
            decode_decrypt(&mut b, &k, 0).unwrap(),
            Bytes::from_static(b"hello")
        );
        // Wrong key: either a padding/garbage magic mismatch or decryption error.
        let other = CryptoKey::generate_aes(UTime::now());
        let ct = encrypt_enc_bl(&k, 0, b"hello").unwrap();
        assert!(decrypt_enc_bl(&other, 0, &ct).is_err());
    }
}
