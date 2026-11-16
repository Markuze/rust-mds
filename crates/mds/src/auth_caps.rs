//! MDS auth caps (`MDSAuthCaps`). The ticket string is the tenant boundary.
//! An empty grant list is unrestricted. A parsed list is checked on the
//! inode's authorization path, including a stray's saved path.

use std::net::IpAddr;

use denc::EntityAddr;

pub const MAY_READ: u32 = 1 << 0;
pub const MAY_WRITE: u32 = 1 << 1;
pub const MAY_EXECUTE: u32 = 1 << 2;
pub const MAY_CHOWN: u32 = 1 << 4;
pub const MAY_CHGRP: u32 = 1 << 5;
pub const MAY_SET_VXATTR: u32 = 1 << 6;
pub const MAY_SNAPSHOT: u32 = 1 << 7;
pub const MAY_FULL: u32 = 1 << 8;

const READ: u32 = 1 << 1;
const WRITE: u32 = 1 << 2;
const SET_VXATTR: u32 = 1 << 3;
const SNAPSHOT: u32 = 1 << 4;
const FULL: u32 = 1 << 5;
const ALL: u32 = 1 << 0;

/// One `allow` grant from an MDS cap string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdsGrant {
    pub spec: u32,
    pub fs_name: String,
    pub path: String,
    pub root_squash: bool,
    pub uid: i64,
    pub gids: Vec<u32>,
    pub network: Option<Net>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Net {
    pub addr: IpAddr,
    pub prefix: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantSet {
    /// `allow *`, `allow`, or a ticket with no MDS cap string.
    Unrestricted,
    Grants(Vec<MdsGrant>),
}

impl GrantSet {
    pub fn parse(text: &str) -> Result<Self, ()> {
        let text = text.trim();
        if text.is_empty() || text == "allow" {
            return Ok(GrantSet::Unrestricted);
        }
        let mut grants = Vec::new();
        for piece in split_grants(text) {
            let grant = parse_grant(piece.trim())?;
            if grant.spec & ALL != 0
                && grant.path.is_empty()
                && grant.uid < 0
                && grant.fs_name.is_empty()
                && !grant.root_squash
                && grant.network.is_none()
            {
                return Ok(GrantSet::Unrestricted);
            }
            grants.push(grant);
        }
        if grants.is_empty() {
            return Err(());
        }
        Ok(GrantSet::Grants(grants))
    }
}

pub fn allows(
    grants: &[MdsGrant],
    fs_name: &str,
    path: &str,
    inode_uid: u32,
    inode_gid: u32,
    inode_mode: u32,
    caller_uid: u32,
    caller_gid: u32,
    caller_gids: &[u64],
    mask: u32,
    new_uid: u32,
    new_gid: u32,
    addr: Option<IpAddr>,
) -> bool {
    let path = path.trim_start_matches('/');
    for grant in grants {
        if let Some(net) = &grant.network {
            let Some(addr) = addr else {
                continue;
            };
            if !net.contains(addr) {
                continue;
            }
        }
        if !grant.matches(fs_name, path, caller_uid, caller_gid, caller_gids) {
            continue;
        }
        if !spec_allows(grant.spec, mask) {
            continue;
        }
        if grant.root_squash && mask & MAY_WRITE != 0 && (caller_uid == 0 || caller_gid == 0) {
            continue;
        }
        if mask & MAY_SET_VXATTR != 0 && grant.spec & (SET_VXATTR | ALL) == 0 {
            continue;
        }
        if mask & MAY_SNAPSHOT != 0 && grant.spec & (SNAPSHOT | ALL) == 0 {
            continue;
        }
        if mask & MAY_FULL != 0 && grant.spec & (FULL | ALL) == 0 {
            continue;
        }
        if grant.uid < 0 {
            return true;
        }
        if mask & MAY_CHOWN != 0 && (new_uid != caller_uid || inode_uid != caller_uid) {
            continue;
        }
        if mask & MAY_CHGRP != 0 && inode_uid != caller_uid {
            continue;
        }
        if mask & MAY_CHGRP != 0 && !gid_ok(grant, caller_gid, caller_gids, new_gid) {
            continue;
        }
        if mode_ok(
            inode_mode,
            inode_uid,
            inode_gid,
            caller_uid,
            grant,
            caller_gid,
            caller_gids,
            mask,
        ) {
            return true;
        }
    }
    false
}

/// The mount root must sit inside some grant path.
pub fn root_allowed(grants: &[MdsGrant], root: &str) -> bool {
    let root = root.trim_start_matches('/');
    grants.iter().any(|grant| path_match(&grant.path, root))
}

impl MdsGrant {
    fn matches(
        &self,
        fs_name: &str,
        path: &str,
        caller_uid: u32,
        caller_gid: u32,
        caller_gids: &[u64],
    ) -> bool {
        if !self.fs_name.is_empty() && self.fs_name != "*" && self.fs_name != fs_name {
            return false;
        }
        if self.uid >= 0 && self.uid != i64::from(caller_uid) {
            return false;
        }
        if self.uid >= 0 && !self.gids.is_empty() {
            let gid_ok = self.gids.iter().any(|gid| u32::from(*gid) == caller_gid)
                || caller_gids
                    .iter()
                    .any(|gid| self.gids.iter().any(|allowed| u64::from(*allowed) == *gid));
            if !gid_ok {
                return false;
            }
        }
        path_match(&self.path, path)
    }
}

fn path_match(grant_path: &str, target: &str) -> bool {
    let grant_path = grant_path.trim_end_matches('/');
    if grant_path.is_empty() {
        return true;
    }
    if !target.starts_with(grant_path) {
        return false;
    }
    target.len() == grant_path.len() || target.as_bytes().get(grant_path.len()) == Some(&b'/')
}

fn spec_allows(spec: u32, mask: u32) -> bool {
    if spec & ALL != 0 {
        return true;
    }
    if mask & (MAY_READ | MAY_EXECUTE) != 0 && spec & READ == 0 {
        return false;
    }
    if mask & MAY_WRITE != 0 && spec & WRITE == 0 {
        return false;
    }
    true
}

fn gid_ok(grant: &MdsGrant, caller_gid: u32, caller_gids: &[u64], new_gid: u32) -> bool {
    u64::from(new_gid) == u64::from(caller_gid)
        || grant.gids.iter().any(|gid| *gid == new_gid)
        || caller_gids.iter().any(|gid| *gid == u64::from(new_gid))
}

fn gid_is(caller_gid: u32, inode_gid: u32) -> bool {
    caller_gid == inode_gid
}

fn mode_ok(
    mode: u32,
    inode_uid: u32,
    inode_gid: u32,
    caller_uid: u32,
    grant: &MdsGrant,
    caller_gid: u32,
    caller_gids: &[u64],
    mask: u32,
) -> bool {
    let group = grant.gids.iter().any(|gid| *gid == inode_gid)
        && (gid_is(caller_gid, inode_gid)
            || caller_gids.iter().any(|gid| *gid == u64::from(inode_gid)));
    let (r, w, x) = if inode_uid == caller_uid {
        (0o400, 0o200, 0o100)
    } else if group {
        (0o040, 0o020, 0o010)
    } else {
        (0o004, 0o002, 0o001)
    };
    (mask & MAY_READ == 0 || mode & r != 0)
        && (mask & MAY_WRITE == 0 || mode & w != 0)
        && (mask & MAY_EXECUTE == 0 || mode & x != 0)
}

impl Net {
    fn contains(&self, addr: IpAddr) -> bool {
        match (self.addr, addr) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let bits = u32::from(self.prefix.min(32));
                let mask = if bits == 0 {
                    0
                } else {
                    u32::MAX << (32 - bits)
                };
                (u32::from(net) & mask) == (u32::from(ip) & mask)
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let bits = usize::from(self.prefix.min(128));
                let net = u128::from(net);
                let ip = u128::from(ip);
                let mask = if bits == 0 {
                    0
                } else {
                    u128::MAX << (128 - bits)
                };
                (net & mask) == (ip & mask)
            }
            _ => false,
        }
    }
}

pub fn addr_of(addr: &EntityAddr) -> Option<IpAddr> {
    addr.addr.map(|socket| socket.ip())
}

fn split_grants(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            i = (i + 1).min(bytes.len());
            continue;
        }
        if bytes[i] == b',' || bytes[i] == b';' {
            let rest = text[i + 1..].trim_start();
            if rest.starts_with("allow") {
                out.push(text[start..i].trim());
                start = text.len() - rest.len();
                i = start;
                continue;
            }
        }
        i += 1;
    }
    out.push(text[start..].trim());
    out
}

fn parse_grant(text: &str) -> Result<MdsGrant, ()> {
    let rest = text.strip_prefix("allow").ok_or(())?.trim_start();
    let (spec, rest) = parse_spec(rest)?;
    let mut grant = MdsGrant {
        spec,
        fs_name: String::new(),
        path: String::new(),
        root_squash: false,
        uid: -1,
        gids: Vec::new(),
        network: None,
    };
    let mut rest = rest.trim_start();
    if let Some(value) = take_eq(&mut rest, "fsname") {
        grant.fs_name = value;
    }
    if let Some(value) = take_eq(&mut rest, "path") {
        grant.path = value.trim_start_matches('/').to_string();
    }
    if take_word(&mut rest, "root_squash") {
        grant.root_squash = true;
    }
    if let Some(value) = take_eq(&mut rest, "uid") {
        grant.uid = i64::from(value.parse::<u32>().map_err(|_| ())?);
    }
    if let Some(value) = take_eq(&mut rest, "gids") {
        for part in value.split(',') {
            grant.gids.push(part.trim().parse().map_err(|_| ())?);
        }
        grant.gids.sort_unstable();
    }
    if take_word(&mut rest, "network") {
        grant.network = Some(parse_net(rest.trim())?);
        rest = "";
    }
    if rest.is_empty() { Ok(grant) } else { Err(()) }
}

fn parse_spec(text: &str) -> Result<(u32, &str), ()> {
    const SPECS: &[(&str, u32)] = &[
        ("*", ALL | READ | WRITE | FULL | SET_VXATTR | SNAPSHOT),
        ("all", ALL | READ | WRITE | FULL | SET_VXATTR | SNAPSHOT),
        ("rwfps", READ | WRITE | FULL | SET_VXATTR | SNAPSHOT),
        ("rwps", READ | WRITE | SET_VXATTR | SNAPSHOT),
        ("rwfp", READ | WRITE | FULL | SET_VXATTR),
        ("rwfs", READ | WRITE | FULL | SNAPSHOT),
        ("rwp", READ | WRITE | SET_VXATTR),
        ("rws", READ | WRITE | SNAPSHOT),
        ("rwf", READ | WRITE | FULL),
        ("rw", READ | WRITE),
        ("r", READ),
    ];
    for (name, bits) in SPECS {
        if let Some(rest) = text.strip_prefix(name) {
            if rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace()) {
                return Ok((*bits, rest));
            }
        }
    }
    Err(())
}

fn take_eq(rest: &mut &str, key: &str) -> Option<String> {
    let trimmed = rest.trim_start();
    let body = trimmed.strip_prefix(key)?.strip_prefix('=')?;
    let (value, next) = if let Some(body) = body.strip_prefix('"') {
        let end = body.find('"')?;
        (body[..end].to_string(), &body[end + 1..])
    } else if let Some(body) = body.strip_prefix('\'') {
        let end = body.find('\'')?;
        (body[..end].to_string(), &body[end + 1..])
    } else {
        let end = body.find(char::is_whitespace).unwrap_or(body.len());
        (body[..end].to_string(), &body[end..])
    };
    *rest = next;
    Some(value)
}

fn take_word(rest: &mut &str, key: &str) -> bool {
    let trimmed = rest.trim_start();
    if let Some(next) = trimmed.strip_prefix(key) {
        if next.is_empty() || next.starts_with(|c: char| c.is_whitespace()) {
            *rest = next;
            return true;
        }
    }
    false
}

fn parse_net(text: &str) -> Result<Net, ()> {
    let (addr, prefix) = text.split_once('/').unwrap_or((text, ""));
    let addr: IpAddr = addr.parse().map_err(|_| ())?;
    let prefix = if prefix.is_empty() {
        if addr.is_ipv4() { 32 } else { 128 }
    } else {
        prefix.parse().map_err(|_| ())?
    };
    Ok(Net { addr, prefix })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tenant_grants_and_refuses_a_bad_string() {
        assert!(matches!(
            GrantSet::parse("allow *").unwrap(),
            GrantSet::Unrestricted
        ));
        assert!(matches!(
            GrantSet::parse("allow").unwrap(),
            GrantSet::Unrestricted
        ));
        let GrantSet::Grants(grants) = GrantSet::parse(
            "allow rw path=/volumes/a uid=1000 gids=1000,1001, allow r path=/volumes/b",
        )
        .unwrap() else {
            panic!("grants");
        };
        assert_eq!(grants[0].path, "volumes/a");
        assert_eq!(grants[0].uid, 1000);
        assert_eq!(grants[0].gids, vec![1000, 1001]);
        assert!(grants[0].spec & WRITE != 0);
        assert_eq!(grants[1].path, "volumes/b");
        assert!(grants[1].spec & WRITE == 0);
        assert!(GrantSet::parse("allow x").is_err());
    }

    #[test]
    fn path_uid_squash_and_vxattr_are_the_tenant_boundary() {
        let GrantSet::Grants(grants) =
            GrantSet::parse("allow rw path=/volumes/a root_squash uid=1000 gids=1000").unwrap()
        else {
            panic!("grant");
        };
        let ok = |path, uid, mask| {
            allows(
                &grants,
                "a",
                path,
                1000,
                1000,
                0o644,
                uid,
                1000,
                &[],
                mask,
                uid,
                1000,
                None,
            )
        };
        assert!(ok("volumes/a/file", 1000, MAY_WRITE));
        assert!(!ok("volumes/b/file", 1000, MAY_READ));
        assert!(!ok("volumes/a/file", 0, MAY_WRITE));
        assert!(!ok("volumes/a/file", 1000, MAY_SET_VXATTR));
        assert!(!allows(
            &grants,
            "a",
            "volumes/a/file",
            1000,
            1000,
            0o644,
            1002,
            1000,
            &[],
            MAY_READ,
            1002,
            1000,
            None
        ));
    }

    #[test]
    fn network_limits_the_grant() {
        let GrantSet::Grants(grants) =
            GrantSet::parse("allow rw path=/volumes/a network 10.0.0.0/8").unwrap()
        else {
            panic!("grant");
        };
        let ip = |s: &str| s.parse().unwrap();
        assert!(allows(
            &grants,
            "a",
            "volumes/a",
            0,
            0,
            0o777,
            1,
            1,
            &[],
            MAY_READ,
            1,
            1,
            Some(ip("10.1.2.3"))
        ));
        assert!(!allows(
            &grants,
            "a",
            "volumes/a",
            0,
            0,
            0o777,
            1,
            1,
            &[],
            MAY_READ,
            1,
            1,
            Some(ip("11.0.0.1"))
        ));
    }

    #[test]
    fn quoted_paths_fsname_and_grant_separators() {
        assert!(matches!(
            GrantSet::parse("allow").unwrap(),
            GrantSet::Unrestricted
        ));
        assert!(matches!(
            GrantSet::parse("allow *").unwrap(),
            GrantSet::Unrestricted
        ));
        let GrantSet::Grants(grants) = GrantSet::parse(
            "allow rw fsname=a path=\"/volumes/a\" ; allow r fsname=b path='/volumes/b'",
        )
        .unwrap() else {
            panic!("grants");
        };
        assert_eq!(grants[0].path, "volumes/a");
        assert_eq!(grants[0].fs_name, "a");
        assert_eq!(grants[1].path, "volumes/b");
        assert!(allows(
            &grants,
            "a",
            "volumes/a/file",
            0,
            0,
            0o644,
            1,
            1,
            &[],
            MAY_WRITE,
            1,
            1,
            None
        ));
        assert!(!allows(
            &grants,
            "other",
            "volumes/a/file",
            0,
            0,
            0o644,
            1,
            1,
            &[],
            MAY_READ,
            1,
            1,
            None
        ));
        assert!(!allows(
            &grants,
            "b",
            "volumes/b/file",
            0,
            0,
            0o644,
            1,
            1,
            &[],
            MAY_WRITE,
            1,
            1,
            None
        ));
        assert!(!path_match("foo", "food"));
        assert!(path_match("foo", "foo/bar"));
    }

    #[test]
    fn ipv6_network_and_a_bad_string_deny() {
        let GrantSet::Grants(grants) =
            GrantSet::parse("allow rw path=/volumes/a network fe80::/10").unwrap()
        else {
            panic!("grant");
        };
        let ip = |s: &str| s.parse().unwrap();
        assert!(allows(
            &grants,
            "a",
            "volumes/a",
            0,
            0,
            0o777,
            1,
            1,
            &[],
            MAY_READ,
            1,
            1,
            Some(ip("fe80::1"))
        ));
        assert!(!allows(
            &grants,
            "a",
            "volumes/a",
            0,
            0,
            0o777,
            1,
            1,
            &[],
            MAY_READ,
            1,
            1,
            Some(ip("2001:db8::1"))
        ));
        assert!(matches!(GrantSet::parse("allow nope").unwrap_err(), ()));
    }
}
