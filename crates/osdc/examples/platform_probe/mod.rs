//! Opt-in real-peer protocol probe. QA alone runs it. No pool or filesystem changes.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use monc::MonClient;
use osdc::ops::{ObjectOperation, cmp_op};
use osdc::{NamespaceScope, ObjectLocator, Objecter, OpError, OpReply, OpRequest, PoolListError};

pub enum Failure {
    Prerequisite(String),
    Check(String),
}

fn failed(message: impl Into<String>) -> Failure {
    Failure::Check(message.into())
}

struct Scratch {
    name: String,
    loc: ObjectLocator,
    owner: Bytes,
    created: bool,
    safe_to_remove: bool,
}

impl Scratch {
    async fn checked(&mut self, o: &Objecter, op: ObjectOperation) -> Result<OpReply, OpError> {
        self.safe_to_remove = false;
        let result = o
            .operate_checked(OpRequest::new(self.loc.clone(), self.name.clone(), op))
            .await;
        // A negative RADOS response is a completed, rejected compound. Other errors
        // can leave the write outcome unknown. Never race cleanup against that write.
        self.safe_to_remove = matches!(&result, Ok(_) | Err(OpError::Rados(_)));
        result
    }
}

fn success(r: &OpReply, count: usize, label: &str) -> Result<(), Failure> {
    println!(
        "{label}: overall={} results={} rvals={:?}",
        r.result,
        r.ops.len(),
        r.ops.iter().map(|op| op.rval).collect::<Vec<_>>()
    );
    if r.result < 0 || r.ops.len() != count || r.ops.iter().any(|op| op.rval < 0) {
        return Err(failed(format!("{label}: invalid checked success")));
    }
    Ok(())
}

async fn state(
    o: &Objecter,
    s: &Scratch,
    data: &[u8],
    tail: &[u8],
    version: u64,
) -> Result<u64, Failure> {
    let mut op = ObjectOperation::new();
    op.read(0, 4096).getxattr("probe_tail");
    let r = o
        .operate_checked(OpRequest::new(s.loc.clone(), s.name.clone(), op))
        .await
        .map_err(|e| failed(format!("state read: {e}")))?;
    success(&r, 2, "state read")?;
    if r.out(0).as_ref() != data
        || r.out(1).as_ref() != tail
        || (version != 0 && r.user_version != version)
    {
        return Err(failed(
            "guard failure changed content, tail, or user version",
        ));
    }
    Ok(r.user_version)
}

async fn guard_failure(
    o: &Objecter,
    s: &mut Scratch,
    bad: ObjectOperation,
    errno: i32,
    data: &[u8],
    tail: &[u8],
    version: u64,
) -> Result<(), Failure> {
    s.safe_to_remove = false;
    let raw = o
        .operate(OpRequest::new(s.loc.clone(), s.name.clone(), bad.clone()))
        .await;
    s.safe_to_remove = raw.is_ok();
    let raw = raw.map_err(|e| failed(format!("raw guard operation: {e}")))?;
    println!(
        "guard rejection: overall={} results={} rvals={:?}",
        raw.result,
        raw.ops.len(),
        raw.ops.iter().map(|op| op.rval).collect::<Vec<_>>()
    );
    if raw.result != errno
        || raw.ops.len() != 3
        || raw.ops.iter().any(|op| op.rval != 0 && op.rval != errno)
    {
        return Err(failed("unexpected real-peer guard result/cardinality"));
    }
    state(o, s, data, tail, version).await?;
    let checked = s.checked(o, bad).await;
    if checked != Err(OpError::Rados(errno)) {
        return Err(failed(
            "checked guard failure did not return the expected errno",
        ));
    }
    state(o, s, data, tail, version).await?;
    Ok(())
}

async fn latest_map(o: &Objecter, monc: &MonClient) -> Result<(), Failure> {
    let (newest, oldest) = monc
        .get_version("osdmap", Duration::from_secs(5))
        .await
        .map_err(|_| failed("get_version(osdmap) failed or timed out"))?;
    let newest = u32::try_from(newest).map_err(|_| failed("OSDMap version exceeds u32"))?;
    if newest == 0 || u64::from(newest) < oldest {
        return Err(failed("invalid monitor version range"));
    }
    let reached = o
        .wait_for_map_timeout(newest, Duration::from_secs(15))
        .await
        .map_err(|e| failed(format!("latest-map checked wait: {e}")))?;
    if reached < newest {
        return Err(failed("checked map wait returned an older epoch"));
    }
    println!(
        "get_version(osdmap): newest={newest} oldest={oldest}; checked wait reached={reached}"
    );
    Ok(())
}

fn scratch(
    o: &Objecter,
    pool: i64,
    prefix: &str,
    namespace: Bytes,
    pg: u32,
) -> Result<Scratch, Failure> {
    let map = o.osdmap();
    let pi = map
        .pool(pool)
        .ok_or_else(|| Failure::Prerequisite("pool is absent".into()))?;
    let mut loc = ObjectLocator::new(pool);
    loc.nspace = namespace;
    for i in 0..8192 {
        let name = format!("{prefix}.{i}");
        if pi.raw_hash_to_pg(pi.hash_key(name.as_bytes(), &loc.nspace)) == pg {
            return Ok(Scratch {
                name,
                loc,
                owner: Bytes::copy_from_slice(prefix.as_bytes()),
                created: false,
                safe_to_remove: true,
            });
        }
    }
    Err(Failure::Prerequisite(
        "could not select bounded unique sentinel names for target PGs".into(),
    ))
}

async fn body(
    o: &Objecter,
    monc: &MonClient,
    pool: i64,
    objects: &mut Vec<Scratch>,
) -> Result<(), Failure> {
    latest_map(o, monc).await?;
    let map = o.osdmap();
    let pi = map
        .pool(pool)
        .ok_or_else(|| Failure::Prerequisite("requested pool is absent".into()))?;
    if pi.pg_num == 0 {
        return Err(failed("pool has zero PGs"));
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| failed("clock is before the Unix epoch"))?
        .as_nanos();
    let prefix = format!("osdc-platform-probe.{}.{nonce}", std::process::id());
    objects.push(scratch(
        o,
        pool,
        &format!("{prefix}.default"),
        Bytes::new(),
        0,
    )?);
    objects.push(scratch(
        o,
        pool,
        &format!("{prefix}.namespaced"),
        Bytes::from(prefix.clone()),
        pi.pg_num - 1,
    )?);
    println!(
        "sentinel placement snapshot: epoch={} PGs={}; target PGs=0,{}",
        map.epoch,
        pi.pg_num,
        pi.pg_num - 1
    );
    for (i, s) in objects.iter_mut().enumerate() {
        // Print only this run's exact candidates. An interrupted/uncertain exclusive
        // create is never added to the cleanup set without its checked success.
        println!(
            "candidate: namespace={:?} name={}",
            String::from_utf8_lossy(&s.loc.nspace),
            s.name
        );
        let mut op = ObjectOperation::new();
        op.create(true)
            .write_full(Bytes::from_static(if i == 0 {
                b"initial"
            } else {
                b"sentinel"
            }))
            .setxattr("probe_owner", s.owner.clone())
            .setxattr("probe_tail", Bytes::from_static(b"0"));
        let r = s.checked(o, op).await.map_err(|e| match e {
            OpError::Rados(-13 | -1) => Failure::Prerequisite("OSD caps must permit create/write/xattrs in both default and non-default namespaces".into()),
            e => failed(format!("exclusive sentinel creation: {e}")),
        })?;
        s.created = true;
        success(&r, 4, "exclusive create+write+owner+tail")?;
    }
    let s = &mut objects[0];
    let mut good = ObjectOperation::new();
    good.cmpxattr_str("probe_owner", cmp_op::EQ, &s.owner)
        .write_full(Bytes::from_static(b"cmp-committed"))
        .setxattr("probe_tail", Bytes::from_static(b"1"));
    let r = s
        .checked(o, good)
        .await
        .map_err(|e| failed(format!("checked CMPXATTR success: {e}")))?;
    success(&r, 3, "checked CMPXATTR+write+tail")?;
    let cmp_version = state(o, s, b"cmp-committed", b"1", r.user_version).await?;
    if cmp_version == 0 {
        return Err(failed("zero user version cannot test ASSERT_VER"));
    }
    let mut bad = ObjectOperation::new();
    bad.cmpxattr_str("probe_owner", cmp_op::EQ, b"not-the-owner")
        .write_full(Bytes::from_static(b"forbidden"))
        .setxattr("probe_tail", Bytes::from_static(b"99"));
    guard_failure(o, s, bad, -125, b"cmp-committed", b"1", cmp_version).await?;
    let mut good = ObjectOperation::new();
    good.assert_version(cmp_version)
        .write_full(Bytes::from_static(b"ver-committed"))
        .setxattr("probe_tail", Bytes::from_static(b"2"));
    let r = s
        .checked(o, good)
        .await
        .map_err(|e| failed(format!("checked ASSERT_VER success: {e}")))?;
    success(&r, 3, "checked ASSERT_VER+write+tail")?;
    if r.user_version <= cmp_version {
        return Err(failed("successful mutation did not advance version"));
    }
    let mut bad = ObjectOperation::new();
    bad.assert_version(cmp_version)
        .write_full(Bytes::from_static(b"forbidden"))
        .setxattr("probe_tail", Bytes::from_static(b"99"));
    guard_failure(o, s, bad, -34, b"ver-committed", b"2", r.user_version).await?;

    let mut cursor = None;
    let mut found = [false; 2];
    let mut empty_continuations = 0;
    let mut restarts = 0;
    for pages in 1..=16384 {
        // A one-entry page forces normal continuation as well as empty-PG progress.
        match o
            .list_pool_page(pool, NamespaceScope::All, cursor.take(), 1, 4096)
            .await
        {
            Ok(page) => {
                empty_continuations += usize::from(page.objects.is_empty() && page.next.is_some());
                for listed in page.objects {
                    for (i, s) in objects.iter().enumerate() {
                        if listed.name == s.name.as_bytes() && listed.namespace == s.loc.nspace {
                            if !listed.locator.is_empty() {
                                return Err(failed("sentinel locator was not empty"));
                            }
                            found[i] = true;
                        }
                    }
                }
                cursor = page.next;
                if cursor.is_none() {
                    println!(
                        "all-PG scan: epoch={} total_pages={pages} restarts={restarts} empty_continuations={empty_continuations} sentinels={found:?}",
                        page.osdmap_epoch
                    );
                    if !found.into_iter().all(|v| v) {
                        return Err(failed("full scan missed a sentinel"));
                    }
                    if empty_continuations == 0 {
                        return Err(Failure::Prerequisite("empty-continuation coverage was not observed; choose an existing allowed pool with empty PGs (no pool parameters were changed)".into()));
                    }
                    return Ok(());
                }
            }
            Err(PoolListError::MapChanged) if restarts < 3 => {
                restarts += 1;
                found = [false; 2];
                empty_continuations = 0;
                latest_map(o, monc).await?;
                // None restarts the entire scan. No conclusions survive a map change.
            }
            Err(PoolListError::Unsupported) => {
                return Err(Failure::Prerequisite(
                    "OSD PGNLS support and SORTBITWISE are required".into(),
                ));
            }
            Err(PoolListError::Io(OpError::Rados(-13 | -1))) => {
                return Err(Failure::Prerequisite(
                    "OSD caps must permit all-namespace pool listing".into(),
                ));
            }
            Err(e) => return Err(failed(format!("native pool listing failed: {e}"))),
        }
    }
    Err(failed("native listing exceeded 16384 page attempts"))
}

async fn cleanup(o: &Objecter, objects: &[Scratch]) -> Result<(), Failure> {
    let mut errors = Vec::new();
    for s in objects {
        if !s.created || !s.safe_to_remove {
            if !s.safe_to_remove {
                eprintln!(
                    "NOT REMOVED (unverified creation or uncertain write): namespace={:?} name={}",
                    String::from_utf8_lossy(&s.loc.nspace),
                    s.name
                );
                errors.push("uncertain outcome; exact candidate retained");
            }
            continue;
        }
        println!(
            "cleanup exact verified object: namespace={:?} name={}",
            String::from_utf8_lossy(&s.loc.nspace),
            s.name
        );
        let mut op = ObjectOperation::new();
        op.cmpxattr_str("probe_owner", cmp_op::EQ, &s.owner)
            .remove();
        match o
            .operate_checked(OpRequest::new(s.loc.clone(), s.name.clone(), op))
            .await
        {
            Ok(r) => success(&r, 2, "checked owner+remove")?,
            Err(_) => errors.push("guarded cleanup failed; exact object may remain"),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(failed(errors.join("; ")))
    }
}

pub async fn run(o: &Objecter, monc: &MonClient, pool: i64) -> Result<(), Failure> {
    let mut objects = Vec::new();
    let result = tokio::time::timeout(Duration::from_secs(120), body(o, monc, pool, &mut objects))
        .await
        .unwrap_or_else(|_| Err(failed("probe exceeded its 120-second work deadline")));
    let clean = tokio::time::timeout(Duration::from_secs(30), cleanup(o, &objects)).await
        .unwrap_or_else(|_| Err(failed("30-second cleanup deadline expired; inspect only the exact candidates printed above")));
    if let Err(e) = clean {
        if let Err(Failure::Check(message) | Failure::Prerequisite(message)) = &result {
            eprintln!("probe before cleanup: {message}");
        }
        return Err(e);
    }
    result
}
