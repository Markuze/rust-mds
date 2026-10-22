//! `ceph-rmds`: the Rust CephFS metadata server.
//!
//! Wiring (filled in as the crates land):
//!
//! - `monc`: monitor session, cephx, MDSMap, beacons; drives the rank state machine.
//! - `msgr`: the client-facing messenger (msgr1 first).
//! - sessions: one per client connection identity, each with its own ordered send queue.
//! - `engine`: the metadata engine for the single shard of this milestone.
//! - `proto`: translation between wire messages and engine calls and effects.

mod config;
mod daemon;
mod fencing;
mod journal;
mod persistence;
mod rados;
mod server;
mod session;
#[cfg(test)]
mod test_backend;

use anyhow::Result;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("RMDS_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cfg = config::from_args(std::env::args().skip(1))?;
    tracing::info!(id = %cfg.id, conf = %cfg.conf_path.display(), mon_host = %cfg.mon_host, "starting");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("rmds")
        .build()?;
    rt.block_on(run(cfg))
}

async fn run(cfg: config::Config) -> Result<()> {
    daemon::run(cfg).await
}
