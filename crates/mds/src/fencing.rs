//! Client fencing follows C++ MDSRank::evict_client: blocklist, fresh map,
//! epoch barrier, then session cleanup. Workers return evidence, never remove rights.

use std::time::Duration;

use denc::EntityAddr;
use monc::MonClient;
use osdc::objecter::Objecter;

pub const CONTROL_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Membership {
    pub gid: u64,
    pub rank: i32,
    pub incarnation: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eviction {
    pub client: u64,
    pub session_generation: u64,
    pub addr: EntityAddr,
}

/// Small test boundary. Production uses the same Objecter/messenger as the daemon.
pub trait FenceBackend: Sync {
    fn raise_barrier(&self, epoch: u32);
    fn is_blocklisted(&self, addr: &EntityAddr) -> bool;
    fn blocklist(
        &self,
        addr: &EntityAddr,
    ) -> impl std::future::Future<Output = Result<(), String>> + Send;
    fn latest_epoch(&self) -> impl std::future::Future<Output = Result<u32, String>> + Send;
    fn latest_epoch_watched(
        &self,
        check: &(impl Fn() -> Result<(), String> + Send + Sync),
    ) -> impl std::future::Future<Output = Result<u32, String>> + Send {
        async {
            check()?;
            let result = self.latest_epoch().await;
            check()?;
            result
        }
    }
    fn wait_epoch(
        &self,
        epoch: u32,
    ) -> impl std::future::Future<Output = Result<u32, String>> + Send;
}

pub struct ClusterFence {
    pub monc: MonClient,
    pub objecter: Objecter,
}

impl FenceBackend for ClusterFence {
    fn raise_barrier(&self, epoch: u32) {
        self.objecter.set_epoch_barrier(epoch);
    }
    fn is_blocklisted(&self, addr: &EntityAddr) -> bool {
        self.objecter.osdmap().is_blocklisted(addr)
    }

    async fn blocklist(&self, addr: &EntityAddr) -> Result<(), String> {
        match tokio::time::timeout(CONTROL_TIMEOUT, self.monc.blocklist(addr)).await {
            Ok(Ok((0, _))) => Ok(()),
            Ok(Ok((r, text))) => Err(format!("blocklist returned {r}: {text}")),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("blocklist timed out".into()),
        }
    }

    async fn latest_epoch(&self) -> Result<u32, String> {
        latest_epoch(&self.monc, &self.objecter).await
    }
    async fn latest_epoch_watched(
        &self,
        check: &(impl Fn() -> Result<(), String> + Send + Sync),
    ) -> Result<u32, String> {
        latest_epoch_checked(&self.monc, &self.objecter, check).await
    }

    async fn wait_epoch(&self, epoch: u32) -> Result<u32, String> {
        self.objecter
            .wait_for_map_timeout(epoch, CONTROL_TIMEOUT)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Query, do not substitute an envelope/cached version after failure.
pub async fn latest_epoch(monc: &MonClient, objecter: &Objecter) -> Result<u32, String> {
    latest_epoch_checked(monc, objecter, &|| Ok(())).await
}

async fn latest_epoch_checked(
    monc: &MonClient,
    objecter: &Objecter,
    check: &(impl Fn() -> Result<(), String> + Send + Sync),
) -> Result<u32, String> {
    check()?;
    let result = monc.get_version("osdmap", CONTROL_TIMEOUT).await;
    check()?;
    let (version, _) = result?;
    let epoch = u32::try_from(version).map_err(|_| "OSDMap epoch overflow".to_string())?;
    check()?;
    let result = tokio::time::timeout(CONTROL_TIMEOUT, objecter.status()).await;
    check()?;
    let status = result.map_err(|_| "Objecter status timed out".to_string())?;
    if epoch == 0 || epoch < objecter.epoch() || epoch < status.latest_map_version {
        return Err("monitor OSDMap version precedes the applied map".into());
    }
    Ok(epoch)
}

/// A failed command is not proof. A fresh map can independently prove an existing
/// entry. Even command success requires observing the exact address/nonce in that map.
pub async fn fence(
    backend: &impl FenceBackend,
    addr: &EntityAddr,
    barrier: u32,
) -> Result<u32, String> {
    fence_checked(backend, addr, barrier, &|| Ok(())).await
}

pub async fn fence_checked(
    backend: &impl FenceBackend,
    addr: &EntityAddr,
    barrier: u32,
    check: &(impl Fn() -> Result<(), String> + Send + Sync),
) -> Result<u32, String> {
    check()?;
    let command = if backend.is_blocklisted(addr) {
        Ok(())
    } else {
        check()?;
        let result = backend.blocklist(addr).await;
        check()?;
        result
    };
    check()?;
    let result = backend.latest_epoch_watched(check).await;
    check()?;
    let epoch = result?.max(barrier);
    check()?;
    let result = backend.wait_epoch(epoch).await;
    check()?;
    let reached = result?;
    if reached < epoch {
        return Err("map wait completed below the fencing epoch".into());
    }
    if !backend.is_blocklisted(addr) {
        return Err(command
            .err()
            .unwrap_or_else(|| "fresh OSDMap lacks the blocklist entry".into()));
    }
    Ok(reached)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[tokio::test]
    async fn e1_watched_fence_stops_after_each_await() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Watched {
            calls: AtomicUsize,
            stop: usize,
        }
        impl FenceBackend for Watched {
            fn raise_barrier(&self, _: u32) {}
            fn is_blocklisted(&self, _: &EntityAddr) -> bool {
                self.calls.load(Ordering::Relaxed) > 0
            }
            async fn blocklist(&self, _: &EntityAddr) -> Result<(), String> {
                self.calls.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            async fn latest_epoch(&self) -> Result<u32, String> {
                self.calls.fetch_add(1, Ordering::Relaxed);
                Ok(9)
            }
            async fn wait_epoch(&self, _: u32) -> Result<u32, String> {
                self.calls.fetch_add(1, Ordering::Relaxed);
                Ok(9)
            }
        }
        let addr = EntityAddr::new(
            denc::AddrType::LEGACY,
            "127.0.0.1:9000".parse().unwrap(),
            33,
        );
        for stop in 1..=3 {
            let backend = Watched {
                calls: AtomicUsize::new(0),
                stop,
            };
            let check = || {
                if backend.calls.load(Ordering::Relaxed) >= backend.stop {
                    Err("authority lost".into())
                } else {
                    Ok(())
                }
            };
            assert!(fence_checked(&backend, &addr, 0, &check).await.is_err());
            assert_eq!(backend.calls.load(Ordering::Relaxed), stop);
        }
    }

    struct Backend {
        command_ok: bool,
        entry: bool,
        wait_ok: bool,
        log: Mutex<Vec<&'static str>>,
    }
    impl FenceBackend for Backend {
        fn raise_barrier(&self, _: u32) {}
        fn is_blocklisted(&self, _: &EntityAddr) -> bool {
            self.log.lock().unwrap().push("entry");
            self.entry && self.log.lock().unwrap().contains(&"wait")
        }
        async fn blocklist(&self, _: &EntityAddr) -> Result<(), String> {
            self.log.lock().unwrap().push("blocklist");
            self.command_ok.then_some(()).ok_or("rejected".into())
        }
        async fn latest_epoch(&self) -> Result<u32, String> {
            self.log.lock().unwrap().push("latest");
            Ok(7)
        }
        async fn wait_epoch(&self, epoch: u32) -> Result<u32, String> {
            assert_eq!(epoch, 9);
            self.log.lock().unwrap().push("wait");
            self.wait_ok.then_some(epoch).ok_or("timeout".into())
        }
    }
    #[tokio::test]
    async fn fencing_requires_fresh_verified_entry_and_successful_wait() {
        let addr = EntityAddr::new(
            denc::AddrType::LEGACY,
            "127.0.0.1:9000".parse().unwrap(),
            33,
        );
        for (command_ok, entry, wait_ok, ok) in [
            (false, false, true, false),
            (true, true, false, false),
            (true, true, true, true),
            (false, true, true, true),
            (true, false, true, false),
        ] {
            let backend = Backend {
                command_ok,
                entry,
                wait_ok,
                log: Mutex::new(vec![]),
            };
            assert_eq!(fence(&backend, &addr, 9).await.is_ok(), ok);
            assert_eq!(
                *backend.log.lock().unwrap(),
                if wait_ok {
                    vec!["entry", "blocklist", "latest", "wait", "entry"]
                } else {
                    vec!["entry", "blocklist", "latest", "wait"]
                }
            );
        }
    }
}
