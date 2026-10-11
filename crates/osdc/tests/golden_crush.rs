//! CRUSH golden tests: maps compiled or built by `crushtool` (see `testdata/gen_crush.py`)
//! and the mappings `crushtool --test --show-mappings` produced for x in 0..1024, per rule,
//! replica count and device reweight set.

mod common;

use std::collections::BTreeMap;

use bytes::{Bytes, BytesMut};
use common::*;
use denc::{Decode, Encode};
use osdc::crush::CrushMap;

fn fnv1a(h: &mut u64, data: &[u8]) {
    for &b in data {
        *h ^= u64::from(b);
        *h = h.wrapping_mul(0x100000001b3);
    }
}

/// The weight vector `CrushTester::test` builds: explicit `--weight` values (scaled like
/// `set_device_weight`), else 0x10000 for devices present in a bucket, else 0.
fn tester_weights(map: &CrushMap, overrides: &[(i32, f32)]) -> Vec<u32> {
    let present = |o: i32| map.buckets.iter().flatten().any(|b| b.items.contains(&o));
    (0..map.max_devices)
        .map(|o| {
            if let Some((_, f)) = overrides.iter().find(|(d, _)| *d == o) {
                ((*f * 65536.0f32) as i32).clamp(0, 0x10000) as u32
            } else if present(o) {
                0x10000
            } else {
                0
            }
        })
        .collect()
}

fn parse_weights(text: &str) -> BTreeMap<u32, Vec<(i32, f32)>> {
    let mut m = BTreeMap::new();
    for line in text.lines() {
        let (id, rest) = line.split_once(' ').unwrap_or((line, ""));
        let v = rest
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| {
                let (d, w) = s.split_once(':').unwrap();
                (d.parse().unwrap(), w.parse().unwrap())
            })
            .collect();
        m.insert(id.trim().parse().unwrap(), v);
    }
    m
}

fn fmt_vec(v: &[i32]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

#[test]
fn crush_mappings_match_crushtool() {
    let dir = testdata().join("crush");
    let mut maps: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "bin")).collect();
    maps.sort();
    assert!(maps.len() >= 10, "fixtures missing in {}", dir.display());
    let mut cases = 0;
    for bin in maps {
        let name = bin.file_stem().unwrap().to_string_lossy().to_string();
        let map = CrushMap::decode(&mut read(&bin)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let weights = parse_weights(&std::fs::read_to_string(bin.with_extension("weights")).unwrap());
        let text = std::fs::read_to_string(bin.with_extension("mappings")).unwrap();
        for line in text.lines() {
            let f: Vec<&str> = line.split(' ').collect();
            let wid: u32 = f[1].parse().unwrap();
            let rule: i32 = f[2].parse().unwrap();
            let numrep: usize = f[3].parse().unwrap();
            let w = tester_weights(&map, &weights[&wid]);
            match f[0] {
                "M" => {
                    let x: i32 = f[4].parse().unwrap();
                    let want = f.get(5).copied().unwrap_or("");
                    let got = map.do_rule(rule, x, numrep, &w, 0);
                    assert_eq!(fmt_vec(&got), want, "{name}: weights {wid} rule {rule} numrep {numrep} x {x}");
                }
                "D" => {
                    let mut h: u64 = 0xcbf29ce484222325;
                    for x in 0..1024 {
                        let got = map.do_rule(rule, x, numrep, &w, 0);
                        fnv1a(&mut h, format!("{x} {}\n", fmt_vec(&got)).as_bytes());
                    }
                    assert_eq!(format!("{h:016x}"), f[4], "{name}: weights {wid} rule {rule} numrep {numrep}: x 0..1024 differ");
                    cases += 1;
                }
                _ => panic!("bad line {line}"),
            }
        }
    }
    assert!(cases > 100, "only {cases} digest cases");
}

/// Decoding keeps every field: re-encoding with the features crushtool used gives the same
/// bytes.
#[test]
fn crush_maps_reencode_identically() {
    let dir = testdata().join("crush");
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_none_or(|e| e != "bin") {
            continue;
        }
        let orig = read(&p);
        let map = CrushMap::decode(&mut orig.clone()).unwrap();
        let mut b = BytesMut::new();
        map.encode(&mut b, denc::features::ALL);
        assert_eq!(Bytes::from(b), orig, "{}", p.display());
    }
    let live = read(&testdata().join("live/crush.bin"));
    let map = CrushMap::decode(&mut live.clone()).unwrap();
    let mut b = BytesMut::new();
    map.encode(&mut b, denc::features::ALL);
    assert_eq!(Bytes::from(b), live);
}
