//! Decoded OSDMap fields against `ceph-dencoder type OSDMap ... dump_json` of the same maps
//! (`testdata/dencoder/*.facts`, made by `gen_dencoder_facts.py`). This covers fields that
//! placement does not use (quotas, tiers, xinfo, uuids, addresses, ratios, profiles, ...).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::*;
use denc::{AddrType, EntityAddr, EntityAddrVec};
use osdc::OsdMap;

fn addr_str(a: &EntityAddr) -> String {
    let sock = a.addr.map(|s| s.to_string()).unwrap_or_else(|| "-".into());
    let prefix = match a.ty {
        AddrType::LEGACY => "v1:",
        AddrType::MSGR2 => "v2:",
        AddrType::ANY => "",
        AddrType(4) => "cidr:",
        _ => "none:",
    };
    format!("{prefix}{sock}/{}", a.nonce)
}

fn addrvec_str(v: &EntityAddrVec) -> String {
    if v.0.is_empty() {
        return "-".into();
    }
    v.0.iter().map(addr_str).collect::<Vec<_>>().join(",")
}

fn kv(rest: &str) -> BTreeMap<String, String> {
    rest.split(' ')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn feq(a: f64, b: &str) -> bool {
    (a - b.parse::<f64>().unwrap()).abs() < 1e-6
}

fn uuid_str(u: &denc::Uuid) -> String {
    let h: String = u.0.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

fn check(m: &OsdMap, facts: &str, what: &str) {
    let mut blocklist = BTreeSet::new();
    let mut range_blocklist = BTreeSet::new();
    let mut pg_temp = 0;
    let mut primary_temp = 0;
    let mut upmap = 0;
    let mut upmap_items = 0;
    let mut upmap_primary = 0;
    for line in facts.lines() {
        let (key, rest) = line.split_once(' ').unwrap();
        match key {
            "epoch" => assert_eq!(m.epoch.to_string(), rest, "{what}"),
            "flags_num" => assert_eq!(m.flags.to_string(), rest, "{what} flags"),
            "crush_version" => assert_eq!(m.crush_version.to_string(), rest, "{what}"),
            "pool_max" => assert_eq!(m.pool_max.to_string(), rest, "{what}"),
            "max_osd" => assert_eq!(m.max_osd.to_string(), rest, "{what}"),
            "fsid" => assert_eq!(uuid_str(&m.fsid), rest, "{what}"),
            "full_ratio" => assert!(feq(f64::from(m.full_ratio), rest), "{what}"),
            "backfillfull_ratio" => assert!(feq(f64::from(m.backfillfull_ratio), rest), "{what}"),
            "nearfull_ratio" => assert!(feq(f64::from(m.nearfull_ratio), rest), "{what}"),
            "require_osd_release" => assert_eq!(m.require_osd_release.name(), rest, "{what}"),
            "require_min_compat_client" => {
                assert_eq!(m.require_min_compat_client.name(), rest, "{what}")
            }
            "pool" => {
                let (id, rest) = rest.split_once(' ').unwrap();
                let id: i64 = id.parse().unwrap();
                let p = &m.pools[&id];
                let f = kv(rest);
                let ours: BTreeMap<&str, String> = [
                    ("name", m.pool_name[&id].clone()),
                    ("type", p.pool_type.to_string()),
                    ("size", p.size.to_string()),
                    ("min_size", p.min_size.to_string()),
                    ("crush_rule", p.crush_rule.to_string()),
                    ("object_hash", p.object_hash.to_string()),
                    ("pg_num", p.pg_num.to_string()),
                    ("pgp_num", p.pgp_num.to_string()),
                    ("pg_num_target", p.pg_num_target.to_string()),
                    ("pgp_num_target", p.pgp_num_target.to_string()),
                    ("pg_num_pending", p.pg_num_pending.to_string()),
                    ("last_change", p.last_change.to_string()),
                    ("last_force_op_resend", p.last_force_op_resend.to_string()),
                    ("flags", p.flags.to_string()),
                    ("snap_seq", p.snap_seq.to_string()),
                    ("snap_epoch", p.snap_epoch.to_string()),
                    ("quota_max_bytes", p.quota_max_bytes.to_string()),
                    ("quota_max_objects", p.quota_max_objects.to_string()),
                    ("tier_of", p.tier_of.to_string()),
                    ("read_tier", p.read_tier.to_string()),
                    ("write_tier", p.write_tier.to_string()),
                    ("stripe_width", p.stripe_width.to_string()),
                    ("expected_num_objects", p.expected_num_objects.to_string()),
                    ("fast_read", p.fast_read.to_string()),
                    (
                        "cache_target_dirty_ratio_micro",
                        p.cache_target_dirty_ratio_micro.to_string(),
                    ),
                    (
                        "cache_target_full_ratio_micro",
                        p.cache_target_full_ratio_micro.to_string(),
                    ),
                    (
                        "min_read_recency_for_promote",
                        p.min_read_recency_for_promote.to_string(),
                    ),
                    (
                        "erasure_code_profile",
                        if p.erasure_code_profile.is_empty() {
                            "-".into()
                        } else {
                            p.erasure_code_profile.clone()
                        },
                    ),
                    ("snaps", p.snaps.len().to_string()),
                    (
                        "apps",
                        if p.application_metadata.is_empty() {
                            "-".into()
                        } else {
                            p.application_metadata
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(",")
                        },
                    ),
                ]
                .into_iter()
                .collect();
                for (k, v) in &ours {
                    let want = f
                        .get(*k)
                        .unwrap_or_else(|| panic!("{what}: {k} missing in facts"));
                    let want = if *k == "fast_read" {
                        if want == "True" {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    } else {
                        want.clone()
                    };
                    assert_eq!(v, &want, "{what}: pool {id} {k}");
                }
            }
            "osd" => {
                let (id, rest) = rest.split_once(' ').unwrap();
                let o: i32 = id.parse().unwrap();
                let i = o as usize;
                let f = kv(rest);
                assert_eq!(
                    u8::from(m.is_up(o)).to_string(),
                    f["up"],
                    "{what}: osd.{o} up"
                );
                assert_eq!(
                    u8::from(m.is_in(o)).to_string(),
                    f["in"],
                    "{what}: osd.{o} in"
                );
                assert!(
                    feq(f64::from(m.osd_weight[i] as f32 / 65536.0), &f["weight"]),
                    "{what}: osd.{o} weight"
                );
                assert!(
                    feq(
                        f64::from(m.primary_affinity(o) as f32 / 65536.0),
                        &f["primary_affinity"]
                    ),
                    "{what}: osd.{o} affinity"
                );
                let info = &m.osd_info[i];
                assert_eq!(info.up_from.to_string(), f["up_from"], "{what}: osd.{o}");
                assert_eq!(info.up_thru.to_string(), f["up_thru"], "{what}: osd.{o}");
                assert_eq!(info.down_at.to_string(), f["down_at"], "{what}: osd.{o}");
                assert_eq!(info.lost_at.to_string(), f["lost_at"], "{what}: osd.{o}");
                assert_eq!(
                    info.last_clean_begin.to_string(),
                    f["last_clean_begin"],
                    "{what}: osd.{o}"
                );
                assert_eq!(
                    info.last_clean_end.to_string(),
                    f["last_clean_end"],
                    "{what}: osd.{o}"
                );
                assert_eq!(uuid_str(&m.osd_uuid[i]), f["uuid"], "{what}: osd.{o} uuid");
                assert_eq!(
                    addrvec_str(&m.client_addrs[i]),
                    f["public"],
                    "{what}: osd.{o} addrs"
                );
            }
            "xinfo" => {
                let (id, rest) = rest.split_once(' ').unwrap();
                let x = &m.osd_xinfo[id.parse::<usize>().unwrap()];
                let f = kv(rest);
                assert_eq!(x.features.to_string(), f["features"], "{what}: xinfo {id}");
                assert_eq!(
                    x.old_weight.to_string(),
                    f["old_weight"],
                    "{what}: xinfo {id}"
                );
                assert_eq!(
                    x.dead_epoch.to_string(),
                    f["dead_epoch"],
                    "{what}: xinfo {id}"
                );
                assert_eq!(
                    x.laggy_interval.to_string(),
                    f["laggy_interval"],
                    "{what}: xinfo {id}"
                );
                assert!(
                    feq(f64::from(x.laggy_probability), &f["laggy_probability"]),
                    "{what}: xinfo {id}"
                );
            }
            "blocklist" => {
                blocklist.insert(rest.to_string());
            }
            "range_blocklist" => {
                range_blocklist.insert(rest.to_string());
            }
            "pg_temp" => {
                let (pg, osds) = rest.split_once(' ').unwrap();
                assert_eq!(
                    m.pg_temp[&parse_pgid(pg)],
                    parse_vec(osds),
                    "{what}: pg_temp {pg}"
                );
                pg_temp += 1;
            }
            "primary_temp" => {
                let (pg, osd) = rest.split_once(' ').unwrap();
                assert_eq!(
                    m.primary_temp[&parse_pgid(pg)].to_string(),
                    osd,
                    "{what}: primary_temp {pg}"
                );
                primary_temp += 1;
            }
            "pg_upmap" => {
                let (pg, osds) = rest.split_once(' ').unwrap();
                assert_eq!(
                    m.pg_upmap[&parse_pgid(pg)],
                    parse_vec(osds),
                    "{what}: pg_upmap {pg}"
                );
                upmap += 1;
            }
            "pg_upmap_items" => {
                let (pg, items) = rest.split_once(' ').unwrap();
                let want: Vec<(i32, i32)> = items
                    .split(',')
                    .map(|p| {
                        let (a, b) = p.split_once('>').unwrap();
                        (a.parse().unwrap(), b.parse().unwrap())
                    })
                    .collect();
                assert_eq!(
                    m.pg_upmap_items[&parse_pgid(pg)],
                    want,
                    "{what}: pg_upmap_items {pg}"
                );
                upmap_items += 1;
            }
            "pg_upmap_primary" => {
                let (pg, osd) = rest.split_once(' ').unwrap();
                assert_eq!(
                    m.pg_upmap_primaries[&parse_pgid(pg)].to_string(),
                    osd,
                    "{what}: pg_upmap_primary {pg}"
                );
                upmap_primary += 1;
            }
            "ec_profile" => {
                let (name, kvs) = rest.split_once(' ').unwrap();
                let prof = &m.erasure_code_profiles[name];
                let ours: Vec<String> = prof.iter().map(|(k, v)| format!("{k}={v}")).collect();
                assert_eq!(ours.join(","), kvs, "{what}: ec profile {name}");
            }
            "removed_snaps_queue" => {
                let (pool, ivs) = rest.split_once(' ').unwrap();
                let q = &m.removed_snaps_queue[&pool.parse::<i64>().unwrap()];
                let ours: Vec<String> = q.iter().map(|(s, l)| format!("{s}~{l}")).collect();
                assert_eq!(ours.join(","), ivs, "{what}: removed_snaps_queue {pool}");
            }
            other => panic!("unknown fact {other}"),
        }
    }
    let ours: BTreeSet<String> = m.blocklist.keys().map(addr_str).collect();
    assert_eq!(ours, blocklist, "{what}: blocklist");
    let ours: BTreeSet<String> = m.range_blocklist.keys().map(addr_str).collect();
    assert_eq!(ours, range_blocklist, "{what}: range blocklist");
    assert_eq!(
        (
            m.pg_temp.len(),
            m.primary_temp.len(),
            m.pg_upmap.len(),
            m.pg_upmap_items.len(),
            m.pg_upmap_primaries.len()
        ),
        (pg_temp, primary_temp, upmap, upmap_items, upmap_primary),
        "{what}: entry counts"
    );
}

#[test]
fn decoded_fields_match_ceph_dencoder() {
    let dir = testdata().join("dencoder");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let Some(stem) = name.strip_suffix(".facts") else {
            continue;
        };
        let (set, epoch) = stem.split_once('.').unwrap();
        let m = load_map(&testdata().join(format!("{set}/full.{epoch}")));
        check(&m, &std::fs::read_to_string(&p).unwrap(), stem);
        n += 1;
    }
    assert!(n >= 6);
}
