//! Plain-text keyring files (`src/auth/KeyRing.cc`, `KeyRing::decode_plaintext` and
//! `set_modifier`):
//!
//! ```text
//! [mds.rust]
//!     key = AQBJ...==
//!     caps mds = "allow"
//!     caps mon = "allow profile mds"
//! ```
//!
//! `key` is the base64 of an encoded `CryptoKey` (type, created, secret); `caps <svc>` values
//! are kept as text; `auid` is ignored. Comments start with `#` or `;`.

use std::collections::BTreeMap;
use std::path::Path;

use crate::{AuthError, AuthName, CryptoKey};

/// One keyring entity.
#[derive(Debug, Clone, Default)]
pub struct KeyringEntry {
    pub key: Option<CryptoKey>,
    pub caps: BTreeMap<String, String>,
}

/// A parsed keyring.
#[derive(Debug, Clone, Default)]
pub struct Keyring {
    pub entries: BTreeMap<AuthName, KeyringEntry>,
}

impl Keyring {
    /// Parse the text form.
    pub fn parse(text: &str) -> Result<Self, AuthError> {
        let mut kr = Keyring::default();
        let mut current: Option<AuthName> = None;
        for (lineno, raw) in text.lines().enumerate() {
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix('[') {
                let name = rest
                    .strip_suffix(']')
                    .ok_or_else(|| {
                        AuthError::Keyring(format!("line {}: bad section header", lineno + 1))
                    })?
                    .trim();
                let n = AuthName::parse(name).ok_or_else(|| {
                    AuthError::Keyring(format!("line {}: bad entity name {name:?}", lineno + 1))
                })?;
                kr.entries.entry(n.clone()).or_default();
                current = Some(n);
                continue;
            }
            let (k, v) = line.split_once('=').ok_or_else(|| {
                AuthError::Keyring(format!("line {}: expected key = value", lineno + 1))
            })?;
            let k = k.split_whitespace().collect::<Vec<_>>().join(" ");
            let v = unquote(v.trim());
            let Some(name) = &current else {
                return Err(AuthError::Keyring(format!(
                    "line {}: value outside a section",
                    lineno + 1
                )));
            };
            let entry = kr.entries.get_mut(name).expect("section exists");
            if k == "key" {
                entry.key = Some(CryptoKey::from_base64(&v)?);
            } else if let Some(svc) = k.strip_prefix("caps ") {
                entry.caps.insert(svc.trim().to_string(), v);
            } else if k == "auid" {
                // ignored, as in C++
            } else {
                return Err(AuthError::Keyring(format!(
                    "line {}: unknown field {k:?}",
                    lineno + 1
                )));
            }
        }
        Ok(kr)
    }

    /// Read and parse a keyring file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AuthError> {
        let p = path.as_ref();
        let text = std::fs::read_to_string(p)
            .map_err(|e| AuthError::Keyring(format!("cannot read {}: {e}", p.display())))?;
        Self::parse(&text)
    }

    /// The secret of `name`.
    pub fn key(&self, name: &AuthName) -> Option<&CryptoKey> {
        self.entries.get(name).and_then(|e| e.key.as_ref())
    }
}

fn strip_comment(line: &str) -> &str {
    // A '#' or ';' outside quotes starts a comment.
    let mut in_quote = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_quote = !in_quote,
            '#' | ';' if !in_quote => return &line[..i],
            _ => {}
        }
    }
    line
}

fn unquote(v: &str) -> String {
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        v[1..v.len() - 1].replace("\\\"", "\"")
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::EntityType;

    #[test]
    fn parses_ceph_authtool_output() {
        // The layout of `ceph-authtool --create-keyring k --gen-key -n mds.rust
        //   --cap mds allow --cap mon 'allow profile mds'`, with a synthetic key.
        let text = "[mds.rust]\n\tkey = AQAA8VNlFc1bBxAAAAECAwQFBgcICQoLDA0ODw==\n\tcaps mds = \"allow\"\n\tcaps mon = \"allow profile mds\"\n";
        let kr = Keyring::parse(text).unwrap();
        let name = AuthName::new(EntityType::MDS, "rust");
        let k = kr.key(&name).unwrap();
        assert_eq!(k.key_type(), crate::crypto::CRYPTO_AES);
        assert_eq!(hex::encode(k.secret()), "000102030405060708090a0b0c0d0e0f");
        assert_eq!(
            k.created(),
            denc::UTime {
                sec: 1_700_000_000,
                nsec: 123_456_789
            }
        );
        assert_eq!(kr.entries[&name].caps["mon"], "allow profile mds");
        assert_eq!(kr.entries[&name].caps["mds"], "allow");
    }

    #[test]
    fn rejects_garbage() {
        assert!(Keyring::parse("key = x\n").is_err());
        assert!(Keyring::parse("[nosuch.x]\n").is_err());
        assert!(Keyring::parse("[client.a]\nkey = !!!\n").is_err());
    }
}
