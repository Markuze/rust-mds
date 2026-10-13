//! Helpers shared by the golden tests: fixture paths and parsers for `osdmaptool` and
//! `crushtool` output.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use bytes::Bytes;
use osdc::{OsdMap, PgId};

pub fn testdata() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

pub fn read(p: &Path) -> Bytes {
    Bytes::from(std::fs::read(p).unwrap_or_else(|e| panic!("read {}: {e}", p.display())))
}

pub fn load_map(p: &Path) -> OsdMap {
    OsdMap::decode(&read(p)).unwrap_or_else(|e| panic!("decode {}: {e}", p.display()))
}

/// Parse `[1,2,3]` (an empty list is `[]`).
pub fn parse_vec(s: &str) -> Vec<i32> {
    let s = s.trim().trim_start_matches('[').trim_end_matches(']');
    if s.trim().is_empty() {
        return Vec::new();
    }
    s.split(',')
        .map(|x| x.trim().parse::<i64>().unwrap() as i32)
        .collect()
}

/// Parse a pgid `pool.seedhex`.
pub fn parse_pgid(s: &str) -> PgId {
    let (p, s) = s.split_once('.').unwrap();
    PgId::new(p.parse().unwrap(), u32::from_str_radix(s, 16).unwrap())
}

/// One line of `osdmaptool --test-map-pgs-dump-all`:
/// `3.0 raw ([1,2,0], p1) up ([1,2,0], p1) acting ([1,2,0], p1)`.
#[derive(Debug, PartialEq, Eq)]
pub struct PgsLine {
    pub pg: PgId,
    pub raw: Vec<i32>,
    pub raw_primary: i32,
    pub up: Vec<i32>,
    pub up_primary: i32,
    pub acting: Vec<i32>,
    pub acting_primary: i32,
}

fn parse_set(s: &str) -> (Vec<i32>, i32) {
    // "([1,2,0], p1)"
    let s = s.trim().trim_start_matches('(').trim_end_matches(')');
    let (v, p) = s.rsplit_once(", p").unwrap();
    (parse_vec(v), p.trim().parse().unwrap())
}

pub fn parse_pgs_dump_all(text: &str) -> Vec<PgsLine> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((pg, rest)) = line.split_once(" raw ") else {
            continue;
        };
        let (raw, rest) = rest.split_once(" up ").unwrap();
        let (up, acting) = rest.split_once(" acting ").unwrap();
        let (raw, raw_primary) = parse_set(raw);
        let (up, up_primary) = parse_set(up);
        let (acting, acting_primary) = parse_set(acting);
        out.push(PgsLine {
            pg: parse_pgid(pg),
            raw,
            raw_primary,
            up,
            up_primary,
            acting,
            acting_primary,
        });
    }
    out
}

/// Lines of `crushtool --test --show-mappings`: `CRUSH rule 0 x 5 [0,1,2]`.
pub fn parse_crush_mappings(text: &str) -> Vec<(i32, i32, Vec<i32>)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("CRUSH rule ") else {
            continue;
        };
        let mut it = rest.splitn(4, ' ');
        let rule: i32 = it.next().unwrap().parse().unwrap();
        assert_eq!(it.next(), Some("x"));
        let x: i32 = it.next().unwrap().parse().unwrap();
        out.push((rule, x, parse_vec(it.next().unwrap())));
    }
    out
}

/// Compare our full placement chain for every pg of every pool against a dump.
pub fn check_pgs(map: &OsdMap, text: &str, what: &str) -> usize {
    let lines = parse_pgs_dump_all(text);
    for l in &lines {
        let (raw, rp) = map.pg_to_raw_osds(l.pg);
        let ua = map.pg_to_up_acting_osds(l.pg);
        assert_eq!(
            (&raw, rp),
            (&l.raw, l.raw_primary),
            "{what}: raw of {}",
            l.pg
        );
        assert_eq!(
            (&ua.up, ua.up_primary),
            (&l.up, l.up_primary),
            "{what}: up of {}",
            l.pg
        );
        assert_eq!(
            (&ua.acting, ua.acting_primary),
            (&l.acting, l.acting_primary),
            "{what}: acting of {}",
            l.pg
        );
    }
    lines.len()
}
