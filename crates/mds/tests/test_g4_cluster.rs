//! The local cluster from `devcluster/local-cluster.sh`.
//!
//! This replaces a vstart/vrunner dependency. The test reads the cluster the
//! script already created. It does not create a filesystem, and it does not
//! stop the monitor, manager, or OSD.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|dir| dir.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn export_value(text: &str, key: &str) -> Option<String> {
    let prefix = format!("export {key}=");
    for line in text.lines() {
        let Some(value) = line.trim().strip_prefix(&prefix) else {
            continue;
        };
        let value = value.split('$').next().unwrap_or(value);
        let value = value.trim().trim_end_matches(':');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn cluster_env() -> Option<(PathBuf, String, String, String)> {
    let env_file = std::env::var("RMDS_LOCAL_ENV")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/tmp/rust-mds-local/env.sh"));
    let text = std::fs::read_to_string(&env_file).ok()?;
    let conf = export_value(&text, "CEPH_CONF")?;
    let bin = export_value(&text, "PATH")?;
    let lib = export_value(&text, "CEPH_LIB")?;
    Some((env_file, conf, bin, lib))
}

fn run(bin: &Path, lib: &str, args: &[&str]) -> std::process::Output {
    let path = std::env::var("PATH").unwrap_or_default();
    let mut command = Command::new(bin);
    command
        .args(args)
        .env(
            "PATH",
            format!("{}:{path}", bin.parent().unwrap().display()),
        )
        .env("LD_LIBRARY_PATH", lib)
        .env("CEPH_LIB", lib);
    command.output().expect("command starts")
}

#[test]
fn local_cluster_script_keeps_filesystem_a_and_the_root_wal() {
    let Some((_env, conf, bin, lib)) = cluster_env() else {
        eprintln!("no local cluster env; skipping");
        return;
    };
    let script = repo().join("devcluster/local-cluster.sh");
    let status = Command::new(&script)
        .arg("status")
        .output()
        .expect("local-cluster.sh starts");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(
        status.status.success(),
        "local-cluster.sh status failed: {text}"
    );
    assert!(
        text.contains("a"),
        "status did not name filesystem a: {text}"
    );

    let ceph = PathBuf::from(&bin).join("ceph");
    let dump = run(&ceph, &lib, &["-c", &conf, "fs", "dump"]);
    let dump_text = String::from_utf8_lossy(&dump.stdout);
    assert!(
        dump.status.success(),
        "fs dump failed: {}",
        String::from_utf8_lossy(&dump.stderr)
    );
    assert!(
        dump_text.contains("Filesystem 'a'") && dump_text.contains("fs_name\ta"),
        "filesystem a is missing from fs dump: {dump_text}"
    );

    let rados = PathBuf::from(&bin).join("rados");
    let listed = run(&rados, &lib, &["-c", &conf, "-p", "cephfs.a.meta", "ls"]);
    let names = String::from_utf8_lossy(&listed.stdout);
    assert!(
        listed.status.success(),
        "rados ls failed: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    assert!(
        names.lines().any(|line| line == "rmds.wal.0.0"),
        "root WAL is missing: {names}"
    );
    assert!(
        names.lines().all(|line| line != "rmds.wal.1.0"),
        "a second shard object is present: {names}"
    );
}
