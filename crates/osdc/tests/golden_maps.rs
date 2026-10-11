//! OSDMap golden tests on two recorded map histories (every epoch's full map and incremental
//! as the monitor stored them, plus `osdmaptool --test-map-pgs-dump-all` of each full map):
//!
//! - `testdata/live`: the dev cluster (`gen_live.sh`), 3 OSDs, CephFS pools created and
//!   recreated, MDS failovers (blocklist entries);
//! - `testdata/privmon`: a private test cluster (`gen_privmon.sh`) whose history covers pool
//!   creation and deletion, an EC pool, device classes, pool snapshots, pg_temp,
//!   primary_temp, pg_upmap, pg_upmap_items, pg_upmap_primary, primary affinity, reweights,
//!   OSDs going down/out and rebooting, blocklists (also ranges), PAUSE flags, weight sets.
//!
//! Every epoch's incremental is kept; full maps (and their pg dumps) for a subset of the
//! epochs. Checks: every map decodes (crcs verified); applying the incrementals from epoch 1
//! reproduces each kept full map exactly; the placement chain matches osdmaptool for every pg
//! of every pool at each kept epoch.

mod common;

use common::*;
use osdc::{Incremental, OsdMap};

/// Epochs with a full map, ascending.
fn full_epochs(set: &str) -> Vec<u32> {
    let dir = testdata().join(set);
    let mut v: Vec<u32> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.unwrap().file_name().to_string_lossy().strip_prefix("full.").and_then(|s| s.parse().ok()))
        .collect();
    v.sort();
    assert!(v.len() > 10, "fixtures missing in {}", dir.display());
    v
}

fn last_epoch(set: &str) -> u32 {
    *full_epochs(set).last().unwrap()
}

fn has_full(set: &str, e: u32) -> bool {
    testdata().join(format!("{set}/full.{e}")).exists()
}

fn full(set: &str, e: u32) -> OsdMap {
    load_map(&testdata().join(format!("{set}/full.{e}")))
}

fn inc(set: &str, e: u32) -> Incremental {
    let p = testdata().join(format!("{set}/inc.{e}"));
    Incremental::decode(&read(&p)).unwrap_or_else(|err| panic!("decode {}: {err}", p.display()))
}

fn check_incremental_chain(set: &str) {
    let last = last_epoch(set);
    let mut m = OsdMap::empty();
    for e in 1..=last {
        let i = inc(set, e);
        assert_eq!(i.epoch, e);
        m.apply_incremental(&i).unwrap_or_else(|err| panic!("{set}: apply inc {e}: {err}"));
        if !has_full(set, e) {
            continue;
        }
        let f = full(set, e);
        assert_eq!(f.epoch, e);
        if m != f {
            // narrow the difference down for the failure message
            assert_eq!(m.pools, f.pools, "{set} epoch {e}: pools");
            assert_eq!(m.osd_state, f.osd_state, "{set} epoch {e}: osd_state");
            assert_eq!(m.osd_info, f.osd_info, "{set} epoch {e}: osd_info");
            assert_eq!(m.osd_xinfo, f.osd_xinfo, "{set} epoch {e}: osd_xinfo");
            assert_eq!(m.crush, f.crush, "{set} epoch {e}: crush");
            assert_eq!(m.blocklist, f.blocklist, "{set} epoch {e}: blocklist");
            assert_eq!(format!("{m:?}"), format!("{f:?}"), "{set} epoch {e}");
        }
    }
}

fn check_placement(set: &str) -> usize {
    let mut total = 0;
    for e in full_epochs(set) {
        let text = std::fs::read_to_string(testdata().join(format!("{set}/pgs.{e}"))).unwrap();
        total += check_pgs(&full(set, e), &text, &format!("{set} epoch {e}"));
    }
    total
}

#[test]
fn live_incrementals_reproduce_full_maps() {
    check_incremental_chain("live");
}

#[test]
fn live_placement_matches_osdmaptool() {
    assert!(check_placement("live") > 150);
}

#[test]
fn privmon_incrementals_reproduce_full_maps() {
    check_incremental_chain("privmon");
}

#[test]
fn privmon_placement_matches_osdmaptool() {
    assert!(check_placement("privmon") > 1500);
}

#[test]
fn privmon_history_covers_features() {
    let maps: Vec<OsdMap> = full_epochs("privmon").into_iter().map(|e| full("privmon", e)).collect();
    assert!(maps.iter().any(|m| !m.pg_temp.is_empty()), "pg_temp");
    assert!(maps.iter().any(|m| !m.primary_temp.is_empty()), "primary_temp");
    assert!(maps.iter().any(|m| !m.pg_upmap.is_empty()), "pg_upmap");
    assert!(maps.iter().any(|m| !m.pg_upmap_items.is_empty()), "pg_upmap_items");
    assert!(maps.iter().any(|m| !m.pg_upmap_primaries.is_empty()), "pg_upmap_primaries");
    assert!(maps.iter().any(|m| m.osd_primary_affinity.is_some()), "primary affinity");
    assert!(maps.iter().any(|m| !m.range_blocklist.is_empty()), "range blocklist");
    assert!(maps.iter().any(|m| m.crush.choose_args.contains_key(&-1)), "compat weight set");
    assert!(maps.iter().any(|m| m.pools.values().any(|p| p.is_erasure())), "ec pool");
    assert!(maps.iter().any(|m| m.test_flag(osdc::osdmap::map_flags::PAUSERD)), "pause");
    assert!(maps.iter().any(|m| m.osd_weight.iter().any(|&w| w != 0 && w != 0x10000)), "partial reweight");
}

/// Blocklist checks on the live history: the MDS instances failed over during the captures
/// are blocklisted by address+nonce (type ANY).
#[test]
fn live_blocklist_lookup() {
    let m = full("live", last_epoch("live"));
    assert!(!m.blocklist.is_empty());
    for a in m.blocklist.keys() {
        let mut probe = *a;
        probe.ty = denc::AddrType::LEGACY;
        assert!(m.is_blocklisted(&probe), "{a}");
        probe.nonce = probe.nonce.wrapping_add(1);
        // a different instance of the same address is not blocklisted (unless the whole IP is)
        let whole_ip = m.blocklist.keys().any(|b| b.nonce == 0 && b.addr.map(|s| s.port()) == Some(0));
        if !whole_ip && a.nonce != 0 {
            assert!(!m.is_blocklisted(&probe), "{probe}");
        }
    }
}

/// Object placement against the monitor's `ceph osd map <pool> <object> [<nspace>]` answers
/// (`testdata/live/objects.tsv`): raw hash pg, pg, up and acting sets, with namespaces.
#[test]
fn live_object_placement_matches_monitor() {
    let text = std::fs::read_to_string(testdata().join("live/objects.tsv")).unwrap();
    let mut n = 0;
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let epoch: u32 = f[0].parse().unwrap();
        let pool: i64 = f[1].parse().unwrap();
        let (ns, name) = (f[2], f[3]);
        let m = full("live", epoch);
        let p = m.map_object(pool, name.as_bytes(), ns.as_bytes()).unwrap();
        assert_eq!(p.raw_pg, parse_pgid(f[4]), "raw pg of {ns}/{name}");
        assert_eq!(p.pg, parse_pgid(f[5]), "pg of {ns}/{name}");
        assert_eq!((p.sets.up.clone(), p.sets.up_primary), (parse_vec(f[6]), f[7].parse().unwrap()), "up of {ns}/{name}");
        assert_eq!((p.sets.acting.clone(), p.sets.acting_primary), (parse_vec(f[8]), f[9].parse().unwrap()), "acting of {ns}/{name}");
        n += 1;
    }
    assert!(n > 100);
}

/// The `MOSDMap` messages the monitor sent the C++ MDS (canonical encoding) and the kernel
/// client (re-encoded for the kernel's features: OSDMap client data v9, older pg_pool_t and
/// CRUSH encodings) in the env agent's msgr1 captures (`testdata/captured`, same cluster
/// history as `testdata/live`).
#[test]
fn captured_mosdmap_messages() {
    use denc::Message;
    use osdc::MOsdMap;
    let mut chain = OsdMap::empty();
    let mut chain_at = std::collections::BTreeMap::new();
    for e in 1..=40 {
        chain.apply_incremental(&inc("live", e)).unwrap();
        if e == 25 || e == 32 {
            chain_at.insert(e, chain.clone());
        }
    }
    for (name, epoch) in [("mosdmap-to-mds", 32u32), ("mosdmap-to-kernel", 25)] {
        let front = read(&testdata().join(format!("captured/{name}.front.bin")));
        let m = MOsdMap::decode_payload(4, front, bytes::Bytes::new(), bytes::Bytes::new()).unwrap();
        assert_eq!((m.first(), m.last()), (epoch, epoch), "{name}");
        assert!(m.incremental_maps.is_empty());
        let map = OsdMap::decode(&m.maps[&epoch]).unwrap();
        assert_eq!(map.epoch, epoch);
        let text = std::fs::read_to_string(testdata().join(format!("captured/{name}.pgs"))).unwrap();
        assert!(check_pgs(&map, &text, name) > 10);
        let want = &chain_at[&epoch];
        if name == "mosdmap-to-mds" {
            assert_eq!(&map, want, "{name}: canonical encoding equals the chain");
        } else {
            // the kernel's encoding drops newer fields; what it carries must agree
            assert_eq!(map.fsid, want.fsid);
            assert_eq!(map.flags, want.flags);
            assert_eq!(map.osd_state, want.osd_state);
            assert_eq!(map.osd_weight, want.osd_weight);
            assert_eq!(map.client_addrs, want.client_addrs);
            assert_eq!(map.blocklist, want.blocklist);
            assert_eq!(map.pool_name, want.pool_name);
            assert_eq!(map.crush.buckets, want.crush.buckets);
            assert_eq!(map.crush.rules, want.crush.rules);
            for (id, p) in &map.pools {
                let w = &want.pools[id];
                assert_eq!((p.pg_num, p.pgp_num, p.size, p.min_size, p.crush_rule, p.flags), (w.pg_num, w.pgp_num, w.size, w.min_size, w.crush_rule, w.flags));
                assert_eq!((p.last_change, p.last_force_op_resend, p.application_metadata.clone()), (w.last_change, w.last_force_op_resend, w.application_metadata.clone()));
            }
        }
    }
}
