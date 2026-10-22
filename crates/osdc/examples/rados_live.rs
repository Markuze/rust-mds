//! Live check of the Rust Objecter against the dev cluster's OSDs.
//!
//! ```text
//! source devcluster/env.sh
//! cargo run -p osdc --example rados_live -- [--pool cephfs.a.meta] [--id rust] [--keyring K] [--inc N]
//! ```
//!
//! Authenticates as `mds.<id>` (keyring `$RMDS_KEYRING`) over msgr1, follows the OSDMap
//! through the MonClient subscription, and runs the RADOS operations the MDS needs against a
//! scratch object in the pool: write_full/read/stat, exclusive create, xattrs, omap
//! values/header/removal, `CMPXATTR` u64 guards, `ASSERT_VER` guards, `TRIMTRUNC`, append,
//! remove, and the errors (`-ENOENT`, `-EEXIST`, `-ECANCELED`, `-ERANGE`). Every op carries
//! `FULL_FORCE`. Exits non-zero on the first unexpected result.
//!
//! `--platform-probe` runs only the bounded get_version/checked-guard/native-listing
//! probe. It does not run the legacy workload or blocklist commands. Exit 2 means
//! prerequisite failure; `--skip-missing-prereqs` changes only that exit to 77.

mod platform_probe;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;

use auth::{AuthContext, AuthName, Keyring};
use bytes::Bytes;
use denc::{EntityName, EntityType, Message};
use monc::{CephConf, MonClient, MonClientConfig, MonEvent};
use msgr::Messenger;
use osdc::Objecter;
use osdc::messages::{CEPH_MSG_OSD_MAP, MOsdMap};
use osdc::msgr_transport::MsgrTransport;
use osdc::objecter::{ObjecterConfig, ObjecterEvent, OpError, OpRequest};
use osdc::ops::{self, ObjectOperation, cmp_op};
use osdc::types::ObjectLocator;

fn arg(name: &str) -> Option<String> {
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        if a == name {
            return it.next();
        }
    }
    None
}

fn has_arg(name: &str) -> bool {
    std::env::args().any(|a| a == name)
}

fn prerequisite(message: &str) -> ! {
    eprintln!("PREREQUISITE: {message}");
    std::process::exit(if has_arg("--skip-missing-prereqs") {
        77
    } else {
        2
    });
}

macro_rules! check {
    ($cond:expr, $($fmt:tt)*) => {
        if !$cond {
            eprintln!("FAIL: {}", format!($($fmt)*));
            std::process::exit(1);
        } else {
            println!("ok: {}", format!($($fmt)*));
        }
    };
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();
    let conf_path = arg("--conf")
        .or_else(|| std::env::var("CEPH_CONF").ok())
        .unwrap_or_else(|| prerequisite("supply --conf or CEPH_CONF"));
    let id = arg("--id").unwrap_or_else(|| "rust".into());
    let pool_name = arg("--pool").unwrap_or_else(|| "cephfs.a.meta".into());
    let keyring_path = arg("--keyring")
        .or_else(|| std::env::var("RMDS_KEYRING").ok())
        .unwrap_or_else(|| prerequisite("supply --keyring or RMDS_KEYRING"));

    let conf = CephConf::load(&conf_path).unwrap_or_else(|_| prerequisite("cannot load ceph.conf"));
    let mons = monc::parse_mon_host_v1(
        &conf
            .get("mds", &id, "mon_host")
            .unwrap_or_else(|| prerequisite("ceph.conf has no mon_host")),
    );
    if mons.is_empty() {
        prerequisite("mon_host has no msgr1 address");
    }
    let keyring = Keyring::load(PathBuf::from(&keyring_path))
        .unwrap_or_else(|_| prerequisite("cannot load keyring"));
    let name = AuthName::new(EntityType::MDS, id.clone());
    let key = keyring
        .key(&name)
        .unwrap_or_else(|| prerequisite("keyring has no key for requested mds id"))
        .clone();
    let auth = AuthContext::new(name.clone(), key, auth::ENTITY_MON | auth::ENTITY_OSD);
    let (msgr, _msgr_events) = Messenger::bind(
        EntityName::new(EntityType::MDS, -1),
        "127.0.0.1:0".parse().unwrap(),
        auth.clone(),
    )
    .await
    .unwrap_or_else(|_| prerequisite("cannot bind the probe messenger"));
    println!("bound {} as {}", msgr.my_addr(), msgr.my_name());
    let (monc, mut mon_events) = MonClient::start(msgr.clone(), MonClientConfig::new(mons));
    let gid = monc
        .wait_authenticated(Duration::from_secs(30))
        .await
        .unwrap_or_else(|_| {
            prerequisite("monitor authentication did not complete within 30 seconds")
        });
    println!("authenticated as {name}, global_id {gid}");

    // client_inc: the MDS uses its MDSMap incarnation; any non-zero value exercises it here
    let inc: u32 = arg("--inc")
        .map(|s| {
            s.parse()
                .unwrap_or_else(|_| prerequisite("--inc must be a u32"))
        })
        .unwrap_or(1);
    let cfg = ObjecterConfig {
        client_inc: inc,
        fsid: monc.fsid(),
        op_timeout: has_arg("--platform-probe").then_some(Duration::from_secs(10)),
        ..ObjecterConfig::default()
    };
    let (objecter, mut obj_events) = Objecter::start(MsgrTransport::new(msgr.clone()), cfg);
    // wiring: monitor maps -> Objecter, Objecter map requests -> monitor subscription
    {
        let objecter = objecter.clone();
        tokio::spawn(async move {
            while let Some(ev) = mon_events.recv().await {
                if let MonEvent::Message(raw) = ev
                    && raw.msg_type == CEPH_MSG_OSD_MAP
                {
                    match MOsdMap::from_raw(raw) {
                        Ok(m) => objecter.handle_osd_map(m),
                        Err(e) => eprintln!("bad MOSDMap: {e}"),
                    }
                }
            }
        });
    }
    {
        let monc = monc.clone();
        tokio::spawn(async move {
            while let Some(ev) = obj_events.recv().await {
                match ev {
                    ObjecterEvent::RequestMap { start, onetime } => monc.subscribe(
                        "osdmap",
                        u64::from(start),
                        if onetime {
                            monc::messages::SUBSCRIBE_ONETIME
                        } else {
                            0
                        },
                    ),
                    ObjecterEvent::MapApplied { epoch } => monc.sub_got("osdmap", u64::from(epoch)),
                    ObjecterEvent::Fenced => eprintln!("FENCED: this instance is blocklisted"),
                    other => println!("objecter event: {other:?}"),
                }
            }
        });
    }
    let epoch = objecter
        .wait_for_map_timeout(1, Duration::from_secs(20))
        .await
        .unwrap_or_else(|_| prerequisite("initial OSDMap wait failed"));
    let map = objecter.osdmap();
    let pool = map
        .pool_id(&pool_name)
        .unwrap_or_else(|| prerequisite("requested pool is absent from the OSDMap"));
    println!(
        "osdmap e{epoch}: pool {pool_name} = {pool}, {} osds up",
        (0..map.max_osd).filter(|o| map.is_up(*o)).count()
    );
    if has_arg("--platform-probe") {
        let result = platform_probe::run(&objecter, &monc, pool).await;
        objecter.shutdown();
        monc.shutdown();
        msgr.shutdown();
        match result {
            Ok(()) => println!("PASS: platform protocol probe"),
            Err(platform_probe::Failure::Prerequisite(message)) => prerequisite(&message),
            Err(platform_probe::Failure::Check(message)) => {
                eprintln!("FAIL: {message}");
                std::process::exit(1);
            }
        }
        return;
    }
    let oloc = ObjectLocator::new(pool);
    let obj = format!("osdc-live-test.{}", std::process::id());
    let obj2 = format!("{obj}.omap");
    let pl = map.map_object(pool, obj.as_bytes(), b"").unwrap();
    println!(
        "{obj} -> pg {} -> acting {:?} primary osd.{}",
        pl.pg, pl.sets.acting, pl.sets.acting_primary
    );

    // write_full / read / stat
    let data = Bytes::from(format!(
        "hello from the rust objecter at {:?}",
        std::time::SystemTime::now()
    ));
    let r = objecter
        .write_full(oloc.clone(), obj.clone(), data.clone())
        .await;
    check!(r.is_ok(), "write_full {obj}: {r:?}");
    let back = objecter.read(oloc.clone(), obj.clone(), 0, 0).await;
    check!(back.as_ref() == Ok(&data), "read back {} bytes", data.len());
    let st = objecter.stat(oloc.clone(), obj.clone()).await;
    check!(
        matches!(st, Ok((n, _)) if n == data.len() as u64),
        "stat size {st:?}"
    );

    // exclusive create of an existing object: -EEXIST
    let mut op = ObjectOperation::new();
    op.create(true);
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj.clone(), op))
        .await
        .unwrap();
    check!(r.result == -17, "create excl on existing -> {}", r.result);

    // compound: create excl + setxattr + omap set + omap header
    let mut vals = BTreeMap::new();
    vals.insert(Bytes::from_static(b"k1"), Bytes::from_static(b"v1"));
    vals.insert(Bytes::from_static(b"k2"), Bytes::from_static(b"value-two"));
    vals.insert(Bytes::from_static(b"k3"), Bytes::from_static(b"3"));
    let mut op = ObjectOperation::new();
    op.create(true)
        .setxattr("parent", Bytes::from_static(b"backtrace-bytes"))
        .setxattr("inline_version", Bytes::from_static(b"5"))
        .omap_set_vals(&vals)
        .omap_set_header(Bytes::from_static(b"fnode-header"));
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj2.clone(), op).with_mtime(denc::UTime::now()))
        .await
        .unwrap();
    check!(
        r.result == 0,
        "create+setxattr+omap_set+omap_set_header -> {}",
        r.result
    );
    let uv = r.user_version;

    // reads: getxattr, omap header, omap vals (paged), by keys
    let mut op = ObjectOperation::new();
    op.getxattr("parent")
        .omap_get_header()
        .omap_get_vals(b"", b"", 2)
        .omap_get_vals_by_keys(&[Bytes::from_static(b"k3")].into_iter().collect())
        .stat();
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj2.clone(), op))
        .await
        .unwrap();
    check!(r.result == 0, "getxattr+omap reads -> {}", r.result);
    check!(&r.out(0)[..] == b"backtrace-bytes", "getxattr parent");
    check!(&r.out(1)[..] == b"fnode-header", "omap_get_header");
    let (page, more) = ops::decode_omap_vals(&r.out(2), 2).unwrap();
    check!(
        page.len() == 2 && more,
        "omap_get_vals page of 2, more={more}"
    );
    let (rest, more) = {
        let mut op = ObjectOperation::new();
        op.omap_get_vals(b"k2", b"", 100);
        let r = objecter
            .operate(OpRequest::new(oloc.clone(), obj2.clone(), op))
            .await
            .unwrap();
        ops::decode_omap_vals(&r.out(0), 100).unwrap()
    };
    check!(
        rest.len() == 1 && rest.contains_key(&Bytes::from_static(b"k3")) && !more,
        "omap_get_vals after k2"
    );
    let byk = ops::decode_omap_map(&r.out(3)).unwrap();
    check!(
        byk.get(&Bytes::from_static(b"k3")).map(|v| &v[..]) == Some(&b"3"[..]),
        "omap_get_vals_by_keys"
    );

    // CMPXATTR u64 guard (uninline): passes when our value > stored, else -ECANCELED
    let mut op = ObjectOperation::new();
    op.cmpxattr_u64("inline_version", cmp_op::GT, 7)
        .write(0, Bytes::from_static(b"uninlined"));
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj2.clone(), op))
        .await
        .unwrap();
    check!(r.result == 0, "cmpxattr u64 GT 7 vs 5 -> {}", r.result);
    let mut op = ObjectOperation::new();
    op.cmpxattr_u64("inline_version", cmp_op::GT, 3)
        .write(0, Bytes::from_static(b"nope"));
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj2.clone(), op))
        .await
        .unwrap();
    check!(
        r.result == -125,
        "cmpxattr u64 GT 3 vs 5 -> {} (-ECANCELED)",
        r.result
    );

    // ASSERT_VER ownership guard
    let mut op = ObjectOperation::new();
    op.stat();
    let cur = objecter
        .operate(OpRequest::new(oloc.clone(), obj2.clone(), op))
        .await
        .unwrap()
        .user_version;
    check!(cur > uv, "user version advanced {uv} -> {cur}");
    let mut op = ObjectOperation::new();
    op.assert_version(cur).omap_rm_keys(
        &[Bytes::from_static(b"k1")]
            .into_iter()
            .collect::<BTreeSet<_>>(),
    );
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj2.clone(), op))
        .await
        .unwrap();
    check!(
        r.result == 0,
        "assert_version {cur} + omap_rm_keys -> {}",
        r.result
    );
    let mut op = ObjectOperation::new();
    op.assert_version(cur).omap_clear();
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj2.clone(), op))
        .await
        .unwrap();
    check!(
        r.result == -34,
        "stale assert_version {cur} -> {} (-ERANGE)",
        r.result
    );

    // TRIMTRUNC (truncate_seq 2 to 4 bytes), then a stale seq is a no-op; append
    let mut op = ObjectOperation::new();
    op.trimtrunc(2, 4);
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj.clone(), op))
        .await
        .unwrap();
    check!(r.result == 0, "trimtrunc seq 2 size 4 -> {}", r.result);
    let st = objecter.stat(oloc.clone(), obj.clone()).await.unwrap();
    check!(st.0 == 4, "size after trimtrunc {}", st.0);
    let mut op = ObjectOperation::new();
    op.trimtrunc(1, 0).append(Bytes::from_static(b"++"));
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj.clone(), op))
        .await
        .unwrap();
    check!(r.result == 0, "stale trimtrunc + append -> {}", r.result);
    let back = objecter
        .read(oloc.clone(), obj.clone(), 0, 0)
        .await
        .unwrap();
    check!(
        back.len() == 6 && &back[4..] == b"++",
        "content after stale trimtrunc + append: {:?}",
        back
    );
    let mut op = ObjectOperation::new();
    op.truncate(1).zero(0, 1);
    let r = objecter
        .operate(OpRequest::new(oloc.clone(), obj.clone(), op))
        .await
        .unwrap();
    check!(r.result == 0, "truncate + zero -> {}", r.result);

    // remove, then -ENOENT
    for o in [&obj, &obj2] {
        let r = objecter
            .remove(oloc.clone(), o.clone(), Default::default())
            .await;
        check!(r.is_ok(), "remove {o}: {r:?}");
    }
    let r = objecter.stat(oloc.clone(), obj.clone()).await;
    check!(r == Err(OpError::Rados(-2)), "stat after remove: {r:?}");
    let r = objecter
        .remove(oloc.clone(), obj.clone(), Default::default())
        .await;
    check!(r == Err(OpError::Rados(-2)), "remove again: {r:?}");
    let st = objecter.status().await;
    println!("objecter status: {st:?}");

    // Monitor command (the MDS client-eviction path): blocklist add and rm a
    // scratch address.
    let scratch = "v1:127.0.0.1:1/33333";
    let (r, rs) = monc
        .command(
            vec![format!(
                "{{\"prefix\":\"osd blocklist\", \"blocklistop\":\"add\",\"addr\":\"{scratch}\"}}"
            )],
            Duration::from_secs(30),
        )
        .await
        .expect("mon command");
    check!(r == 0, "mon command blocklist add {scratch}: {r} {rs}");
    let (r, rs) = monc
        .command(
            vec![format!(
                "{{\"prefix\":\"osd blocklist\", \"blocklistop\":\"rm\",\"addr\":\"{scratch}\"}}"
            )],
            Duration::from_secs(30),
        )
        .await
        .expect("mon command");
    check!(r == 0, "mon command blocklist rm {scratch}: {r} {rs}");

    println!("PASS");
    objecter.shutdown();
    monc.shutdown();
    msgr.shutdown();
}
