//! A minimal `ceph.conf` reader: sections, `key = value`, `;`/`#` comments, keys with spaces
//! or underscores treated alike (`mon host` = `mon_host`). Lookup order for a daemon
//! `type.id`: `[type.id]`, `[type]`, `[global]`.
//!
//! `mon_host` accepts the forms of `MonMap::build_initial` / `parse_str`: address vectors
//! `[v2:127.0.0.1:3300,v1:127.0.0.1:6789]`, bare `ip:port` (legacy unless the port is 3300),
//! bare `ip` (v1 port 6789), `v1:ip:port/nonce`, separated by commas, spaces or semicolons.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;

/// Parsed configuration file.
#[derive(Debug, Clone, Default)]
pub struct CephConf {
    pub sections: BTreeMap<String, BTreeMap<String, String>>,
}

fn norm_key(k: &str) -> String {
    k.trim().split([' ', '_', '\t']).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("_")
}

impl CephConf {
    pub fn parse(text: &str) -> Self {
        let mut c = CephConf::default();
        let mut section = "global".to_string();
        for raw in text.lines() {
            let line = match raw.find(['#', ';']) {
                Some(i) => &raw[..i],
                None => raw,
            }
            .trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].trim().to_string();
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().trim_matches('"').to_string();
                c.sections.entry(section.clone()).or_default().insert(norm_key(k), v);
            }
        }
        c
    }

    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        Ok(Self::parse(&std::fs::read_to_string(path)?))
    }

    /// Look up `key` for daemon `type.id`, with `$type`, `$id`, `$name`, `$cluster` expanded.
    pub fn get(&self, ty: &str, id: &str, key: &str) -> Option<String> {
        let k = norm_key(key);
        let name = format!("{ty}.{id}");
        for s in [name.as_str(), ty, "global"] {
            if let Some(v) = self.sections.get(s).and_then(|m| m.get(&k)) {
                let mut v = v.replace("$cluster", "ceph").replace("$type", ty).replace("$id", id).replace("$name", &name);
                // $mds_data and similar references to other keys.
                if let Some(start) = v.find('$') {
                    let rest = &v[start + 1..];
                    let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
                    let var = &rest[..end];
                    if var != key
                        && let Some(sub) = self.get(ty, id, var)
                    {
                        v = format!("{}{}{}", &v[..start], sub, &rest[end..]);
                    }
                }
                return Some(v);
            }
        }
        None
    }
}

/// Parse a `mon_host` value into msgr1 socket addresses (the v1 entry of each address vector,
/// or bare addresses that are not on the msgr2 port 3300).
pub fn parse_mon_host_v1(s: &str) -> Vec<SocketAddr> {
    let mut out = Vec::new();
    let mut rest = s.trim();
    while !rest.is_empty() {
        rest = rest.trim_start_matches([',', ' ', ';', '\t']);
        if rest.is_empty() {
            break;
        }
        if let Some(r) = rest.strip_prefix('[') {
            // An address vector.
            let end = r.find(']').unwrap_or(r.len());
            let inner = &r[..end];
            for a in inner.split(',') {
                let a = a.trim();
                if let Some(v1) = a.strip_prefix("v1:")
                    && let Some(sa) = parse_one(v1, 6789)
                {
                    out.push(sa);
                }
            }
            rest = r.get(end + 1..).unwrap_or("");
        } else {
            let end = rest.find([',', ' ', ';', '\t']).unwrap_or(rest.len());
            let item = &rest[..end];
            if let Some(v1) = item.strip_prefix("v1:") {
                if let Some(sa) = parse_one(v1, 6789) {
                    out.push(sa);
                }
            } else if item.starts_with("v2:") {
                // msgr2 only.
            } else if let Some(sa) = parse_one(item, 6789)
                && sa.port() != 3300
            {
                out.push(sa);
            }
            rest = &rest[end..];
        }
    }
    out
}

/// `ip`, `ip:port`, `[v6]:port`, with an optional `/nonce` suffix.
fn parse_one(s: &str, default_port: u16) -> Option<SocketAddr> {
    let s = s.split('/').next()?.trim();
    if let Ok(sa) = s.parse::<SocketAddr>() {
        return Some(sa);
    }
    if let Ok(ip) = s.trim_matches(['[', ']']).parse::<IpAddr>() {
        return Some(SocketAddr::new(ip, default_port));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mon_host_forms() {
        assert_eq!(parse_mon_host_v1("[v2:127.0.0.1:3300,v1:127.0.0.1:6789]"), vec!["127.0.0.1:6789".parse().unwrap()]);
        assert_eq!(
            parse_mon_host_v1("[v2:10.0.0.1:3300,v1:10.0.0.1:6789] [v2:10.0.0.2:3300,v1:10.0.0.2:6789]"),
            vec!["10.0.0.1:6789".parse().unwrap(), "10.0.0.2:6789".parse::<SocketAddr>().unwrap()]
        );
        assert_eq!(parse_mon_host_v1("10.0.0.1, 10.0.0.2:6790"), vec!["10.0.0.1:6789".parse().unwrap(), "10.0.0.2:6790".parse::<SocketAddr>().unwrap()]);
        assert_eq!(parse_mon_host_v1("v1:1.2.3.4:6789/0"), vec!["1.2.3.4:6789".parse().unwrap()]);
        assert!(parse_mon_host_v1("v2:1.2.3.4:3300").is_empty());
        assert!(parse_mon_host_v1("1.2.3.4:3300").is_empty());
    }

    #[test]
    fn conf_lookup() {
        let c = CephConf::parse(
            "[global]\nfsid = abc\nmon host = 1.2.3.4 ; comment\nkeyring = /k/$cluster.$name.keyring\n[mds]\nmds_data = /d/$id\n[mds.rust]\nmds join fs = a\n",
        );
        assert_eq!(c.get("mds", "rust", "fsid").as_deref(), Some("abc"));
        assert_eq!(c.get("mds", "rust", "mon_host").as_deref(), Some("1.2.3.4"));
        assert_eq!(c.get("mds", "rust", "mds_join_fs").as_deref(), Some("a"));
        assert_eq!(c.get("mds", "rust", "keyring").as_deref(), Some("/k/ceph.mds.rust.keyring"));
        assert_eq!(c.get("mds", "rust", "mds_data").as_deref(), Some("/d/rust"));
        assert_eq!(c.get("mds", "other", "mds_join_fs"), None);
    }
}
