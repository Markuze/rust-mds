//! Command line and `ceph.conf` handling.
//!
//! The command line accepts the flags cephadm and Rook pass to `ceph-mds`
//! (`-i/--id`, `-n/--name`, `-c/--conf`, `--keyring`, `-f`, `-d`, `--setuser`, `--setgroup`,
//! `--default-log-*`, `--mon-host`, `--fsid`, ...); flags the daemon does not use are accepted
//! and ignored so the same command lines work.
//!
//! `ceph.conf` is an INI file. Keys are case-insensitive and treat spaces, underscores and
//! dashes alike. Lookups go from the most specific section to the least:
//! `[mds.<id>]`, `[mds]`, `[global]`. Values may contain the metavariables `$cluster`,
//! `$type`, `$id`, `$name` and `$host`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Normalize a key: lower case, `-` and ` ` become `_`.
fn norm_key(k: &str) -> String {
    k.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

/// A parsed `ceph.conf`.
#[derive(Debug, Default, Clone)]
pub struct CephConf {
    sections: HashMap<String, HashMap<String, String>>,
}

impl CephConf {
    pub fn parse(text: &str) -> Result<Self> {
        let mut conf = CephConf::default();
        let mut section = String::from("global");
        for (lineno, raw) in text.lines().enumerate() {
            let line = raw.split(['#', ';']).next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix('[') {
                let Some(name) = rest.strip_suffix(']') else {
                    bail!("ceph.conf line {}: bad section header", lineno + 1);
                };
                section = name.trim().to_ascii_lowercase();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                bail!("ceph.conf line {}: expected key = value", lineno + 1);
            };
            let v = v.trim().trim_matches('"').to_string();
            conf.sections
                .entry(section.clone())
                .or_default()
                .insert(norm_key(k), v);
        }
        Ok(conf)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text)
    }

    /// Look a key up for daemon `mds.<id>`: `[mds.<id>]`, then `[mds]`, then `[global]`.
    pub fn get(&self, id: &str, key: &str) -> Option<&str> {
        let key = norm_key(key);
        [format!("mds.{id}"), "mds".to_string(), "global".to_string()]
            .iter()
            .find_map(|s| {
                self.sections
                    .get(s)
                    .and_then(|m| m.get(&key))
                    .map(String::as_str)
            })
    }
}

/// Expand `$cluster`, `$type`, `$id`, `$name` and `$host` in a configuration value.
pub fn expand_meta(value: &str, cluster: &str, id: &str) -> String {
    let host = hostname();
    value
        .replace("$cluster", cluster)
        .replace("$type", "mds")
        .replace("$name", &format!("mds.{id}"))
        .replace("$id", id)
        .replace("$host", &host)
}

fn hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}

/// Daemon settings after combining the command line and `ceph.conf`.
#[derive(Debug, Clone)]
pub struct Config {
    /// Daemon id: the `<id>` of `mds.<id>`.
    pub id: String,
    pub conf_path: PathBuf,
    pub keyring: PathBuf,
    /// `mon_host` as written in the configuration (or `--mon-host`).
    pub mon_host: String,
    /// Address to bind; the port may be 0 to pick one from `ms_bind_port_min..=max`.
    pub public_addr: Option<SocketAddr>,
    pub bind_port_min: u16,
    pub bind_port_max: u16,
    /// `mds_join_fs`.
    pub join_fs: Option<String>,
    pub conf: CephConf,
    pub persistence: PersistenceConfig,
}

#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    pub reconnect_timeout: std::time::Duration,
    pub limits: store::Limits,
    pub op_timeout: std::time::Duration,
    pub checkpoint_interval: std::time::Duration,
    pub checkpoint_records: u64,
}
impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            reconnect_timeout: std::time::Duration::from_secs(45),
            limits: store::Limits::default(),
            op_timeout: crate::persistence::OP_TIMEOUT,
            checkpoint_interval: std::time::Duration::from_secs(30),
            checkpoint_records: 16,
        }
    }
}

/// Parse the command line (without the program name).
pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Config> {
    let mut id: Option<String> = None;
    let mut conf_path: Option<PathBuf> = None;
    let mut keyring: Option<PathBuf> = None;
    let mut cluster = String::from("ceph");
    let mut mon_host: Option<String> = None;
    let mut public_addr: Option<String> = None;
    let mut join_fs: Option<String> = None;

    let mut it = args.into_iter().peekable();
    while let Some(arg) = it.next() {
        // Accept both "--flag value" and "--flag=value".
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| -> Result<String> {
            match inline.clone() {
                Some(v) => Ok(v),
                None => it.next().with_context(|| format!("{name} needs a value")),
            }
        };
        match flag.as_str() {
            "-i" | "--id" => id = Some(value(&flag)?),
            "-n" | "--name" => {
                let n = value(&flag)?;
                id = Some(n.strip_prefix("mds.").unwrap_or(&n).to_string());
            }
            "-c" | "--conf" => conf_path = Some(PathBuf::from(value(&flag)?)),
            "-k" | "--keyring" => keyring = Some(PathBuf::from(value(&flag)?)),
            "--cluster" => cluster = value(&flag)?,
            "-m" | "--mon-host" | "--mon_host" => mon_host = Some(value(&flag)?),
            "--public-addr" | "--public_addr" => public_addr = Some(value(&flag)?),
            "--mds-join-fs" | "--mds_join_fs" => join_fs = Some(value(&flag)?),
            // Accepted and ignored: foreground/debug switches and the logging, user, group and
            // identity options cephadm and Rook pass.
            "-f" | "--foreground" | "-d" => {}
            "--setuser"
            | "--setgroup"
            | "--fsid"
            | "--mon-initial-members"
            | "--mon_initial_members"
            | "--log-to-stderr"
            | "--err-to-stderr"
            | "--default-log-to-file"
            | "--default-log-to-stderr"
            | "--default-err-to-stderr"
            | "--default-log-stderr-prefix"
            | "--default-mon-cluster-log-to-file"
            | "--default-mon-cluster-log-to-stderr"
            | "--log-file"
            | "--log_file"
            | "--debug-mds"
            | "--debug-ms" => {
                if inline.is_none() && it.peek().is_some_and(|n| !n.starts_with('-')) {
                    it.next();
                }
            }
            other if other.starts_with("--") => {
                // Unknown long option: skip its value if one follows.
                if inline.is_none() && it.peek().is_some_and(|n| !n.starts_with('-')) {
                    it.next();
                }
            }
            other => bail!("unexpected argument {other:?}"),
        }
    }

    let id = id.context("missing -i <id>")?;
    let conf_path = conf_path
        .or_else(|| std::env::var_os("CEPH_CONF").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(format!("/etc/ceph/{cluster}.conf")));
    let conf = if conf_path.exists() {
        CephConf::load(&conf_path)?
    } else {
        CephConf::default()
    };

    let keyring = match keyring {
        Some(k) => k,
        None => PathBuf::from(expand_meta(
            conf.get(&id, "keyring")
                .unwrap_or("/var/lib/ceph/mds/$cluster-$id/keyring"),
            &cluster,
            &id,
        )),
    };
    let mon_host = match mon_host.or_else(|| conf.get(&id, "mon_host").map(str::to_string)) {
        Some(m) => m,
        None => bail!(
            "no mon_host: pass -m or set mon_host in {}",
            conf_path.display()
        ),
    };
    let public_addr = public_addr
        .or_else(|| conf.get(&id, "public_addr").map(str::to_string))
        .map(|s| parse_bind_addr(&s))
        .transpose()?;
    let port = |key: &str, default: u16| -> Result<u16> {
        conf.get(&id, key)
            .map(|v| v.parse().with_context(|| format!("{key} = {v}")))
            .unwrap_or(Ok(default))
    };
    let join_fs = join_fs.or_else(|| conf.get(&id, "mds_join_fs").map(str::to_string));
    let number = |key: &str, default: u64| -> Result<u64> {
        conf.get(&id, key)
            .map(|value| value.parse().with_context(|| format!("{key} = {value}")))
            .unwrap_or(Ok(default))
    };
    let mut persistence = PersistenceConfig {
        op_timeout: std::time::Duration::from_secs(number("rmds_osd_op_timeout", 30)?),
        checkpoint_interval: std::time::Duration::from_secs(number(
            "rmds_checkpoint_interval",
            30,
        )?),
        checkpoint_records: number("rmds_checkpoint_records", 16)?,
        ..Default::default()
    };
    if let Some(value) = conf.get(&id, "mds_reconnect_timeout") {
        let seconds: f64 = value.parse().context("invalid mds_reconnect_timeout")?;
        persistence.reconnect_timeout = std::time::Duration::try_from_secs_f64(seconds)
            .context("invalid mds_reconnect_timeout")?;
    }
    if persistence.reconnect_timeout.is_zero()
        || persistence
            .reconnect_timeout
            .checked_mul(2)
            .and_then(|d| std::time::Instant::now().checked_add(d))
            .is_none()
    {
        bail!("mds_reconnect_timeout must be positive, finite and support a 2R deadline");
    }
    persistence.limits.max_record_bytes = usize::try_from(number(
        "rmds_max_record_bytes",
        persistence.limits.max_record_bytes as u64,
    )?)?;
    persistence.limits.max_wal_bytes = usize::try_from(number(
        "rmds_max_wal_bytes",
        persistence.limits.max_wal_bytes as u64,
    )?)?;
    persistence.limits.max_batch_bytes = persistence
        .limits
        .max_batch_bytes
        .max(persistence.limits.max_record_bytes);
    persistence.limits.max_checkpoint_generations = number("rmds_max_checkpoint_generations", 64)?;
    if persistence.op_timeout.is_zero()
        || persistence.checkpoint_interval.is_zero()
        || persistence.checkpoint_records == 0
        || persistence.limits.max_record_bytes <= store::framing::OVERHEAD
        || persistence.limits.max_record_bytes > 16 << 20
        || persistence.limits.max_wal_bytes / persistence.limits.max_record_bytes < 8
        || persistence.limits.max_checkpoint_generations < 2
    {
        bail!("invalid bounded persistence settings");
    }

    Ok(Config {
        bind_port_min: port("ms_bind_port_min", 6800)?,
        bind_port_max: port("ms_bind_port_max", 7568)?,
        id,
        conf_path,
        keyring,
        mon_host,
        public_addr,
        join_fs,
        conf,
        persistence,
    })
}

/// Accept `ip`, `ip:port`, `v1:ip:port/nonce` or `[v1:...]` forms; the port defaults to 0.
fn parse_bind_addr(s: &str) -> Result<SocketAddr> {
    let s = s.trim().trim_start_matches('[').trim_end_matches(']');
    let s = s.split(',').next().unwrap_or(s);
    let s = s
        .trim_start_matches("v1:")
        .trim_start_matches("v2:")
        .trim_start_matches("any:");
    let s = s.split('/').next().unwrap_or(s);
    if let Ok(a) = s.parse::<SocketAddr>() {
        return Ok(a);
    }
    let ip: std::net::IpAddr = s.parse().with_context(|| format!("bad address {s:?}"))?;
    Ok(SocketAddr::new(ip, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conf_lookup_order_and_key_normalization() {
        let c = CephConf::parse(
            "[global]\nmon host = [v2:127.0.0.1:3300,v1:127.0.0.1:6789]\nkeyring = /etc/ceph/$cluster.keyring\n\
             [mds]\nms bind port min = 6900\n[mds.rust]\nkeyring = /x/$id.keyring\n",
        )
        .unwrap();
        assert_eq!(
            c.get("rust", "mon_host"),
            Some("[v2:127.0.0.1:3300,v1:127.0.0.1:6789]")
        );
        assert_eq!(c.get("rust", "keyring"), Some("/x/$id.keyring"));
        assert_eq!(c.get("a", "keyring"), Some("/etc/ceph/$cluster.keyring"));
        assert_eq!(c.get("rust", "ms-bind-port-min"), Some("6900"));
        assert_eq!(
            expand_meta("/x/$cluster-$id", "ceph", "rust"),
            "/x/ceph-rust"
        );
    }

    #[test]
    fn cephadm_style_command_line() {
        let dir = std::env::temp_dir().join(format!("rmds-conf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let conf = dir.join("ceph.conf");
        std::fs::write(&conf, "[global]\nmon_host = 127.0.0.1:6789\n").unwrap();
        let args = [
            "-n",
            "mds.rust",
            "-f",
            "--setuser",
            "ceph",
            "--setgroup",
            "ceph",
            "--default-log-to-file=false",
            "-c",
            conf.to_str().unwrap(),
            "--keyring",
            "/k",
        ];
        let cfg = from_args(args.iter().map(|s| s.to_string())).unwrap();
        assert_eq!(cfg.id, "rust");
        assert_eq!(cfg.keyring, PathBuf::from("/k"));
        assert_eq!(cfg.mon_host, "127.0.0.1:6789");
        assert_eq!((cfg.bind_port_min, cfg.bind_port_max), (6800, 7568));
    }

    #[test]
    fn bind_address_forms() {
        assert_eq!(
            parse_bind_addr("127.0.0.1").unwrap(),
            "127.0.0.1:0".parse().unwrap()
        );
        assert_eq!(
            parse_bind_addr("[v1:10.0.0.1:6801/123]").unwrap(),
            "10.0.0.1:6801".parse().unwrap()
        );
    }

    #[test]
    fn g3_reconnect_timeout_requires_positive_finite_representable_window() {
        let dir =
            PathBuf::from("/tmp/opencode").join(format!("rmds-g3-conf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let conf = dir.join("ceph.conf");
        let load = |value: Option<&str>| {
            let mut text = "[global]\nmon_host = 127.0.0.1:6789\n[mds.rust]\n".to_string();
            if let Some(value) = value {
                text.push_str(&format!("mds_reconnect_timeout = {value}\n"));
            }
            std::fs::write(&conf, text).unwrap();
            from_args([
                "-i".to_string(),
                "rust".to_string(),
                "-c".to_string(),
                conf.to_str().unwrap().to_string(),
            ])
        };
        assert_eq!(
            load(None).unwrap().persistence.reconnect_timeout,
            std::time::Duration::from_secs(45)
        );
        assert_eq!(
            load(Some("0.5")).unwrap().persistence.reconnect_timeout,
            std::time::Duration::from_millis(500)
        );
        for value in ["0", "-1", "NaN", "inf", "1e300", "0.0000000001", "invalid"] {
            assert!(
                load(Some(value)).is_err(),
                "invalid timeout accepted: {value}"
            );
        }
        std::fs::remove_file(conf).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
