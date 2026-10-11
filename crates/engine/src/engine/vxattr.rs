//! Parsing and formatting helpers for the `ceph.*` virtual xattrs
//! (`Server::parse_layout_vxattr*`, `parse_quota_vxattr`,
//! `src/include/cephfs/keys_and_values.h`, `src/common/strtol.cc`).

use std::collections::BTreeMap;

use crate::errno::Errno;
use crate::types::FileLayout;

/// `Server::is_ceph_vxattr`.
pub(crate) fn is_ceph_vxattr(name: &str) -> bool {
    name.starts_with("ceph.dir.layout")
        || name.starts_with("ceph.file.layout")
        || name.starts_with("ceph.quota")
        || matches!(
            name,
            "ceph.quiesce.block"
                | "ceph.dir.subvolume"
                | "ceph.dir.pin"
                | "ceph.dir.pin.random"
                | "ceph.dir.pin.distributed"
                | "ceph.dir.charmap"
                | "ceph.dir.normalization"
                | "ceph.dir.encoding"
                | "ceph.dir.casesensitive"
                | "ceph.dir.subvolume.snaps.visible"
        )
}

/// `Server::is_ceph_dir_vxattr`.
pub(crate) fn is_ceph_dir_vxattr(name: &str) -> bool {
    matches!(
        name,
        "ceph.dir.layout"
            | "ceph.dir.layout.json"
            | "ceph.dir.layout.object_size"
            | "ceph.dir.layout.stripe_unit"
            | "ceph.dir.layout.stripe_count"
            | "ceph.dir.layout.pool"
            | "ceph.dir.layout.pool_name"
            | "ceph.dir.layout.pool_id"
            | "ceph.dir.layout.pool_namespace"
            | "ceph.dir.pin"
            | "ceph.dir.pin.random"
            | "ceph.dir.pin.distributed"
            | "ceph.dir.charmap"
            | "ceph.dir.normalization"
            | "ceph.dir.encoding"
            | "ceph.dir.casesensitive"
            | "ceph.dir.subvolume"
            | "ceph.dir.subvolume.snaps.visible"
    )
}

/// `Server::is_ceph_file_vxattr`.
pub(crate) fn is_ceph_file_vxattr(name: &str) -> bool {
    matches!(
        name,
        "ceph.file.layout"
            | "ceph.file.layout.json"
            | "ceph.file.layout.object_size"
            | "ceph.file.layout.stripe_unit"
            | "ceph.file.layout.stripe_count"
            | "ceph.file.layout.pool"
            | "ceph.file.layout.pool_name"
            | "ceph.file.layout.pool_id"
            | "ceph.file.layout.pool_namespace"
    )
}

/// `Server::is_allowed_ceph_xattr`: `ceph.*` names stored as plain xattrs.
pub(crate) fn is_allowed_plain_ceph_xattr(name: &[u8]) -> bool {
    !name.starts_with(b"ceph.")
        || name == b"ceph.mirror.info"
        || name == b"ceph.mirror.dirty_snap_id"
}

/// The `keys_and_values` grammar: `pair (' ' pair)*` with
/// `key = [a-zA-Z_][a-zA-Z_0-9]*`, `value = [a-zA-Z0-9-_.]+`, the whole
/// input consumed. Later duplicates do not replace earlier keys.
pub(crate) fn parse_keys_and_values(s: &str) -> Option<BTreeMap<String, String>> {
    let mut m = BTreeMap::new();
    if s.is_empty() {
        return None;
    }
    for (idx, pair) in s.split(' ').enumerate() {
        let _ = idx;
        let (k, v) = pair.split_once('=')?;
        let mut kc = k.chars();
        let first = kc.next()?;
        if !(first.is_ascii_alphabetic() || first == '_')
            || !kc.all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return None;
        }
        if v.is_empty()
            || !v
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return None;
        }
        m.entry(k.to_string()).or_insert_with(|| v.to_string());
    }
    Some(m)
}

/// `boost::lexical_cast<unsigned>`, decimal digits only.
pub(crate) fn parse_u32(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// `boost::lexical_cast<int64_t>` / `strict_strtoll(.., 10)`.
pub(crate) fn parse_i64(s: &str) -> Option<i64> {
    let digits = s.strip_prefix(['-', '+']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// `strict_strtob`: `true`/`false` (any case) or an integer (non-zero is true).
pub(crate) fn parse_bool(s: &str) -> Option<bool> {
    if s.eq_ignore_ascii_case("true") {
        return Some(true);
    }
    if s.eq_ignore_ascii_case("false") {
        return Some(false);
    }
    let v = parse_i64(s)?;
    if v < i64::from(i32::MIN) || v > i64::from(i32::MAX) {
        return None;
    }
    Some(v != 0)
}

/// `strict_iec_cast<int64_t>`: an integer with an optional IEC or SI prefix
/// (`K`, `M`, `G`, `T`, `P`, `E`, optionally followed by `i` and/or `B`),
/// all meaning powers of 1024.
pub(crate) fn parse_iec_i64(s: &str) -> Option<i64> {
    if s.is_empty() {
        return None;
    }
    let split = s
        .find(|c: char| !(c.is_ascii_digit() || c == '-' || c == '+'))
        .unwrap_or(s.len());
    let (n, mut unit) = s.split_at(split);
    let mut shift = 0u32;
    if !unit.is_empty() {
        if unit.len() > 1 && unit.ends_with('B') {
            unit = &unit[..unit.len() - 1];
        }
        if unit.len() > 2 {
            return None;
        }
        let ub = unit.as_bytes();
        if ub.len() == 2 && ub[1] != b'i' {
            return None;
        }
        if ub.len() == 2 && ub[0] == b'B' {
            return None;
        }
        shift = match ub[0] {
            b'K' => 10,
            b'M' => 20,
            b'G' => 30,
            b'T' => 40,
            b'P' => 50,
            b'E' => 60,
            b'B' => 0,
            _ => return None,
        };
    }
    let v = parse_i64(n)?;
    if shift >= 64 {
        return None;
    }
    if v < (i64::MIN >> shift) || v > (i64::MAX >> shift) {
        return None;
    }
    Some(v << shift)
}

/// A JSON value, just enough for `layout.json`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    /// Numbers keep their text (`JSONObj::get_data`), parsed by the reader.
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    /// The text the C++ `JSONDecoder` hands to the integer parsers.
    pub(crate) fn data(&self) -> Option<&str> {
        match self {
            Json::Num(s) | Json::Str(s) => Some(s),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn ws(&mut self) {
        while self.pos < self.s.len() && matches!(self.s[self.pos], b' ' | b'\t' | b'\n' | b'\r') {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    fn expect(&mut self, b: u8) -> Option<()> {
        self.ws();
        if self.peek()? == b {
            self.pos += 1;
            Some(())
        } else {
            None
        }
    }

    fn value(&mut self) -> Option<Json> {
        self.ws();
        match self.peek()? {
            b'{' => {
                self.pos += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.peek()? == b'}' {
                    self.pos += 1;
                    return Some(Json::Obj(m));
                }
                loop {
                    self.ws();
                    let Json::Str(k) = self.string()? else {
                        return None;
                    };
                    self.expect(b':')?;
                    let v = self.value()?;
                    m.insert(k, v);
                    self.ws();
                    match self.peek()? {
                        b',' => self.pos += 1,
                        b'}' => {
                            self.pos += 1;
                            return Some(Json::Obj(m));
                        }
                        _ => return None,
                    }
                }
            }
            b'[' => {
                self.pos += 1;
                let mut v = Vec::new();
                self.ws();
                if self.peek()? == b']' {
                    self.pos += 1;
                    return Some(Json::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    self.ws();
                    match self.peek()? {
                        b',' => self.pos += 1,
                        b']' => {
                            self.pos += 1;
                            return Some(Json::Arr(v));
                        }
                        _ => return None,
                    }
                }
            }
            b'"' => self.string(),
            b't' => self.lit("true", Json::Bool(true)),
            b'f' => self.lit("false", Json::Bool(false)),
            b'n' => self.lit("null", Json::Null),
            b'-' | b'0'..=b'9' => {
                let start = self.pos;
                while self.pos < self.s.len()
                    && matches!(
                        self.s[self.pos],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.pos += 1;
                }
                Some(Json::Num(
                    String::from_utf8_lossy(&self.s[start..self.pos]).into_owned(),
                ))
            }
            _ => None,
        }
    }

    fn lit(&mut self, word: &str, v: Json) -> Option<Json> {
        if self.s[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Some(v)
        } else {
            None
        }
    }

    fn string(&mut self) -> Option<Json> {
        if self.peek()? != b'"' {
            return None;
        }
        self.pos += 1;
        let mut out = String::new();
        loop {
            let b = self.peek()?;
            self.pos += 1;
            match b {
                b'"' => return Some(Json::Str(out)),
                b'\\' => {
                    let e = self.peek()?;
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex =
                                std::str::from_utf8(self.s.get(self.pos..self.pos + 4)?).ok()?;
                            let cp = u32::from_str_radix(hex, 16).ok()?;
                            self.pos += 4;
                            out.push(char::from_u32(cp)?);
                        }
                        _ => return None,
                    }
                }
                _ => {
                    // Copy raw UTF-8 bytes.
                    let start = self.pos - 1;
                    let mut end = self.pos;
                    while end < self.s.len() && self.s[end] != b'"' && self.s[end] != b'\\' {
                        end += 1;
                    }
                    out.push_str(std::str::from_utf8(&self.s[start..end]).ok()?);
                    self.pos = end;
                }
            }
        }
    }
}

/// Parse a JSON document; trailing garbage is an error.
pub(crate) fn parse_json(s: &str) -> Option<Json> {
    let mut p = JsonParser {
        s: s.as_bytes(),
        pos: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.pos != p.s.len() {
        return None;
    }
    Some(v)
}

/// Pool lookup used by the layout parsers.
pub(crate) trait PoolLookup {
    fn pool_by_name(&self, name: &str) -> Option<i64>;
    fn pool_exists(&self, id: i64) -> bool;
}

/// Apply one layout vxattr (`field` is what follows `layout`, e.g. `""`
/// for the composite form, `"pool"`, `"json"`) to `layout`
/// (`Server::parse_layout_vxattr_string`/`_json`). Unknown pool names give
/// ENOENT for `pool` (the caller turns it into EINVAL), EINVAL otherwise;
/// unknown fields give ENODATA.
pub(crate) fn apply_layout_field(
    layout: &mut FileLayout,
    field: &str,
    value: &str,
    pools: &dyn PoolLookup,
) -> Result<(), Errno> {
    match field {
        "" => {
            let m = parse_keys_and_values(value).ok_or(Errno::EINVAL)?;
            for (k, v) in m {
                apply_layout_field(layout, &k, &v, pools)?;
            }
        }
        "object_size" => layout.object_size = parse_u32(value).ok_or(Errno::EINVAL)?,
        "stripe_unit" => layout.stripe_unit = parse_u32(value).ok_or(Errno::EINVAL)?,
        "stripe_count" => layout.stripe_count = parse_u32(value).ok_or(Errno::EINVAL)?,
        "pool" => {
            layout.pool_id = match parse_u32(value) {
                Some(id) => i64::from(id),
                None => pools.pool_by_name(value).ok_or(Errno::ENOENT)?,
            }
        }
        "pool_id" => layout.pool_id = parse_i64(value).ok_or(Errno::EINVAL)?,
        "pool_name" => layout.pool_id = pools.pool_by_name(value).ok_or(Errno::EINVAL)?,
        "pool_namespace" => layout.pool_ns = value.to_string(),
        "json" => {
            let Some(Json::Obj(m)) = parse_json(value) else {
                return Err(Errno::EINVAL);
            };
            let num = |k: &str| -> Result<u32, Errno> {
                m.get(k)
                    .and_then(|v| v.data())
                    .and_then(parse_u32)
                    .ok_or(Errno::EINVAL)
            };
            layout.object_size = num("object_size")?;
            layout.stripe_unit = num("stripe_unit")?;
            layout.stripe_count = num("stripe_count")?;
            if let Some(ns) = m.get("pool_namespace") {
                layout.pool_ns = ns.data().ok_or(Errno::EINVAL)?.to_string();
            }
            let pool_id = match m.get("pool_id") {
                Some(v) => v.data().and_then(parse_i64).ok_or(Errno::EINVAL)?,
                None => 0,
            };
            let pool_name = match m.get("pool_name") {
                Some(v) => v.data().ok_or(Errno::EINVAL)?.to_string(),
                None => String::new(),
            };
            layout.pool_id = if !pool_name.is_empty() {
                pools.pool_by_name(&pool_name).ok_or(Errno::EINVAL)?
            } else if pool_id >= 0 && pools.pool_exists(pool_id) {
                pool_id
            } else {
                return Err(Errno::EINVAL);
            };
        }
        _ => return Err(Errno::ENODATA),
    }
    Ok(())
}

/// The client-style composite layout string (`ceph_vxattrcb_layout`).
pub(crate) fn layout_string(l: &FileLayout, pool_name: Option<&str>) -> String {
    let mut s = format!(
        "stripe_unit={} stripe_count={} object_size={} pool=",
        l.stripe_unit, l.stripe_count, l.object_size
    );
    match pool_name {
        Some(n) => s.push_str(n),
        None => s.push_str(&l.pool_id.to_string()),
    }
    if !l.pool_ns.is_empty() {
        s.push_str(" pool_namespace=");
        s.push_str(&l.pool_ns);
    }
    s
}

/// The C++ GETVXATTR JSON layout text (exact spacing).
pub(crate) fn layout_json(l: &FileLayout, pool_name: Option<&str>, inheritance: &str) -> String {
    format!(
        "{{\"stripe_unit\": {}, \"stripe_count\": {}, \"object_size\": {}, \"pool_name\": \"{}\", \"pool_id\": {}, \"pool_namespace\": \"{}\", \"inheritance\": \"@{}\"}}",
        l.stripe_unit,
        l.stripe_count,
        l.object_size,
        pool_name.unwrap_or(""),
        l.pool_id as u64,
        l.pool_ns,
        inheritance
    )
}

/// Default `operator<<(double)` formatting for the common cases.
pub(crate) fn format_double(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Pools;
    impl PoolLookup for Pools {
        fn pool_by_name(&self, name: &str) -> Option<i64> {
            match name {
                "data" => Some(2),
                "fast" => Some(3),
                _ => None,
            }
        }
        fn pool_exists(&self, id: i64) -> bool {
            id == 2 || id == 3
        }
    }

    #[test]
    fn keys_and_values() {
        let m = parse_keys_and_values("stripe_unit=65536 pool=data").unwrap();
        assert_eq!(m["stripe_unit"], "65536");
        assert_eq!(m["pool"], "data");
        assert!(parse_keys_and_values("a=1 ").is_none());
        assert!(parse_keys_and_values("1a=1").is_none());
        assert!(parse_keys_and_values("a=b c").is_none());
        assert!(parse_keys_and_values("").is_none());
        let m = parse_keys_and_values("a=1 a=2").unwrap();
        assert_eq!(m["a"], "1");
    }

    #[test]
    fn numbers_and_bools() {
        assert_eq!(parse_iec_i64("10G"), Some(10 << 30));
        assert_eq!(parse_iec_i64("1Ti"), Some(1 << 40));
        assert_eq!(parse_iec_i64("100MiB"), Some(100 << 20));
        assert_eq!(parse_iec_i64("5KB"), Some(5 << 10));
        assert_eq!(parse_iec_i64("123"), Some(123));
        assert_eq!(parse_iec_i64("-1"), Some(-1));
        assert_eq!(parse_iec_i64("1X"), None);
        assert_eq!(parse_iec_i64("1Bi"), None);
        assert_eq!(parse_iec_i64(""), None);
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("2"), Some(true));
        assert_eq!(parse_bool("yes"), None);
        assert_eq!(parse_u32("+1"), None);
        assert_eq!(parse_i64("+5"), Some(5));
    }

    #[test]
    fn layout_fields() {
        let mut l = FileLayout::default_for_pool(2);
        apply_layout_field(
            &mut l,
            "",
            "stripe_unit=1048576 stripe_count=2 pool=fast",
            &Pools,
        )
        .unwrap();
        assert_eq!((l.stripe_unit, l.stripe_count, l.pool_id), (1 << 20, 2, 3));
        assert_eq!(
            apply_layout_field(&mut l, "pool", "nosuch", &Pools),
            Err(Errno::ENOENT)
        );
        assert_eq!(
            apply_layout_field(&mut l, "pool_name", "nosuch", &Pools),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            apply_layout_field(&mut l, "bogus", "1", &Pools),
            Err(Errno::ENODATA)
        );
        apply_layout_field(&mut l, "pool", "2", &Pools).unwrap();
        assert_eq!(l.pool_id, 2);
        apply_layout_field(
            &mut l,
            "json",
            r#"{"object_size": 8388608, "stripe_unit": "4194304", "stripe_count": 2, "pool_name": "fast", "pool_namespace": "ns"}"#,
            &Pools,
        )
        .unwrap();
        assert_eq!(
            (l.object_size, l.stripe_unit, l.stripe_count, l.pool_id),
            (8 << 20, 4 << 20, 2, 3)
        );
        assert_eq!(l.pool_ns, "ns");
        assert_eq!(
            apply_layout_field(&mut l, "json", r#"{"object_size": 1}"#, &Pools),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            apply_layout_field(&mut l, "json", "not json", &Pools),
            Err(Errno::EINVAL)
        );
    }

    #[test]
    fn formats() {
        let l = FileLayout::default_for_pool(3);
        assert_eq!(
            layout_json(&l, Some("cephfs.a.data"), "default"),
            r#"{"stripe_unit": 4194304, "stripe_count": 1, "object_size": 4194304, "pool_name": "cephfs.a.data", "pool_id": 3, "pool_namespace": "", "inheritance": "@default"}"#
        );
        assert_eq!(
            layout_string(&l, Some("p")),
            "stripe_unit=4194304 stripe_count=1 object_size=4194304 pool=p"
        );
        assert_eq!(format_double(0.0), "0");
        assert_eq!(format_double(0.005), "0.005");
        assert!(parse_json(r#"{"a": [1, {"b": null}], "c": "x\"yA"}"#).is_some());
        assert_eq!(parse_json(r#""x\"yA""#), Some(Json::Str("x\"yA".into())));
    }

    #[test]
    fn names() {
        assert!(is_ceph_vxattr("ceph.dir.layout.pool"));
        assert!(is_ceph_vxattr("ceph.quota.max_bytes"));
        assert!(!is_ceph_vxattr("ceph.dir.rbytes"));
        assert!(is_ceph_dir_vxattr("ceph.dir.subvolume"));
        assert!(is_ceph_file_vxattr("ceph.file.layout.json"));
        assert!(is_allowed_plain_ceph_xattr(b"user.x"));
        assert!(!is_allowed_plain_ceph_xattr(b"ceph.dir.rctime"));
    }
}
