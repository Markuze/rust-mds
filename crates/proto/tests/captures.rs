//! Golden checks against real traffic between the C++ MDS (Ceph 20.2) and the kernel client /
//! ceph-fuse, captured by `testdata/capture.sh` on the dev cluster (msgr1 connections; the pcap
//! files are parsed by `common` and `proto::wire::parse_msgr1`).
//!
//! Every message in either direction must decode. Where the encoding is deterministic the
//! re-encoding must reproduce the captured bytes: replies (with `InodeStat` at the v8 that
//! Ceph 20.2 sends), session messages from the MDS, cap messages (at their v12), requests, cap
//! releases and metrics. Tests skip when a capture is missing.

mod common;

use std::collections::BTreeMap;

use denc::Message;
use proto::wire::{Msgr1Conversation, Msgr1Message, parse_msgr1};
use proto::*;

/// msgr1 conversations between a client and an MDS: from `<captures>/<name>.pcap` when the
/// dev cluster's capture exists, else from the corpus extracted from it into `tests/data`.
fn mds_conversations(name: &str) -> Option<Vec<Msgr1Conversation>> {
    let path = common::captures_dir().join(format!("{name}.pcap"));
    if !path.exists() {
        let corpus = common::corpus_path(name);
        if corpus.exists() {
            return Some(vec![common::read_corpus(&corpus)]);
        }
        eprintln!(
            "{} and {} missing; skipping",
            path.display(),
            corpus.display()
        );
        return None;
    }
    Some(pcap_conversations(name, &path))
}

fn pcap_conversations(name: &str, path: &std::path::Path) -> Vec<Msgr1Conversation> {
    let mut out = Vec::new();
    for c in common::tcp_connections(path) {
        // msgr2 (crc mode): the capture filter only admits the MDS's v2 port, so every
        // connection is a client<->MDS one.
        if c.client_to_server.starts_with(b"ceph v2\n") {
            let from_client = common::msgr2_messages(&c.client_to_server)
                .unwrap_or_else(|e| panic!("{name}: msgr2: {e}"));
            let from_server = common::msgr2_messages(&c.server_to_client)
                .unwrap_or_else(|e| panic!("{name}: msgr2: {e}"));
            out.push(Msgr1Conversation {
                features: 0,
                client_type: 8,
                from_client,
                from_server,
                truncated: false,
            });
            continue;
        }
        if !c.client_to_server.starts_with(wire::BANNER)
            || !c.server_to_client.starts_with(wire::BANNER)
        {
            continue;
        }
        let conv = parse_msgr1(&c.client_to_server, &c.server_to_client).unwrap_or_else(|e| {
            panic!("{name}: msgr1 parse of {:?}->{:?}: {e}", c.client, c.server)
        });
        let server_is_mds = conv.from_server.iter().any(|m| m.header.src_type == 2);
        if server_is_mds && conv.client_type == 8 {
            out.push(conv);
        }
    }
    assert!(
        !out.is_empty(),
        "{name}: no client<->MDS msgr1 connection found"
    );
    out
}

/// Refresh `tests/data/<name>.msgs` from the dev cluster's captures:
/// `cargo test -p proto --test captures extract_corpus -- --ignored`.
#[test]
#[ignore]
fn extract_corpus() {
    for name in ["kernel-v1", "kernel-v1-mdsv1", "fuse-v1", "fuse-v2"] {
        let path = common::captures_dir().join(format!("{name}.pcap"));
        if !path.exists() {
            eprintln!("{name}: no capture");
            continue;
        }
        let convs = pcap_conversations(name, &path);
        let out = common::corpus_path(name);
        common::write_corpus(&out, &convs);
        eprintln!("{name}: wrote {}", out.display());
    }
}

/// The corpus files must hold the same messages as the captures they came from.
#[test]
fn corpus_matches_captures() {
    for name in ["kernel-v1", "kernel-v1-mdsv1", "fuse-v1", "fuse-v2"] {
        let path = common::captures_dir().join(format!("{name}.pcap"));
        let corpus = common::corpus_path(name);
        if !path.exists() || !corpus.exists() {
            continue;
        }
        let from_pcap = pcap_conversations(name, &path);
        let from_corpus = common::read_corpus(&corpus);
        let n: usize = from_pcap
            .iter()
            .map(|c| c.from_client.len() + c.from_server.len())
            .sum();
        assert_eq!(
            n,
            from_corpus.from_client.len() + from_corpus.from_server.len(),
            "{name}: refresh the corpus (extract_corpus)"
        );
    }
}

/// What a capture contained, for the coverage assertions.
#[derive(Default, Debug)]
struct Seen {
    request_ops: BTreeMap<MdsOp, usize>,
    reply_ops: BTreeMap<MdsOp, usize>,
    session_ops: BTreeMap<String, usize>,
    caps_from_client: BTreeMap<(String, u16), usize>,
    caps_from_mds: BTreeMap<(String, u16), usize>,
    other: BTreeMap<u16, usize>,
}

fn hexdiff(a: &[u8], b: &[u8]) -> String {
    let i = a
        .iter()
        .zip(b)
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let lo = i.saturating_sub(16);
    format!(
        "lengths {} vs {}, first difference at {i}\n  ours:     {}\n  captured: {}",
        a.len(),
        b.len(),
        hex::encode(&a[lo..(i + 32).min(a.len())]),
        hex::encode(&b[lo..(i + 32).min(b.len())])
    )
}

fn assert_same(what: &str, ours: &[u8], captured: &[u8]) {
    assert!(
        ours == captured,
        "{what}: re-encoding differs: {}",
        hexdiff(ours, captured)
    );
}

fn check_client_message(m: &Msgr1Message, seen: &mut Seen) {
    let raw = m.raw.clone();
    let what = format!(
        "client msg seq {} type {:#x} v{}",
        m.header.seq, raw.msg_type, raw.version
    );
    let decoded = ClientMessage::from_raw(raw.clone()).unwrap_or_else(|e| panic!("{what}: {e}"));
    match decoded.unwrap_or_else(|| panic!("{what}: not a client message type")) {
        ClientMessage::Session(s) => {
            *seen
                .session_ops
                .entry(format!("c:{}", s.op.name()))
                .or_default() += 1;
            assert!(s.op.is_client_op(), "{what}: {:?}", s.op);
            // The head is copied verbatim; v7 must decode to the same message after a
            // re-encode (the kernel's metadata keys are not in map order, so bytes may differ).
            let again = MClientSession::from_raw(s.to_raw(0)).unwrap();
            assert_eq!(again, s, "{what}");
            assert_same(&what, &s.to_raw(0).front[..28], &raw.front[..28]);
        }
        ClientMessage::Request(r) => {
            *seen.request_ops.entry(r.op()).or_default() += 1;
            assert_eq!(r.tid, raw.tid);
            assert_eq!(r.head.version, 3, "{what}: request head version");
            let p = r.encode_payload(0);
            assert_same(&what, &p.front, &raw.front);
            assert_same(&format!("{what} data"), &p.data, &raw.data);
            if r.op().is_new_inode() {
                assert_ne!(r.head.owner_uid, u32::MAX, "{what}: owner");
                r.initial_xattrs()
                    .unwrap_or_else(|e| panic!("{what}: initial xattrs: {e}"));
            }
        }
        ClientMessage::Caps(c) => {
            *seen
                .caps_from_client
                .entry((c.op.name().to_string(), raw.version))
                .or_default() += 1;
            let p = c.encode_payload_version(raw.version);
            assert_same(&what, &p.front, &raw.front);
            assert_same(&format!("{what} middle"), &p.middle, &raw.middle);
        }
        ClientMessage::CapRelease(c) => {
            *seen.other.entry(raw.msg_type).or_default() += 1;
            assert_same(&what, &c.encode_payload(0).front, &raw.front);
        }
        ClientMessage::Metrics(mm) => {
            *seen.other.entry(raw.msg_type).or_default() += 1;
            for x in &mm.metrics {
                assert!(
                    !matches!(x, ClientMetric::Unknown { .. }),
                    "{what}: unparsed metric {x:?}"
                );
            }
            assert_same(&what, &mm.encode_payload(0).front, &raw.front);
        }
        ClientMessage::Lease(l) => {
            *seen.other.entry(raw.msg_type).or_default() += 1;
            let ours = l.encode_payload(0).front;
            assert_same(&what, &ours, &raw.front[..ours.len()]);
        }
        ClientMessage::Reconnect(r) => {
            *seen.other.entry(raw.msg_type).or_default() += 1;
            assert_same(&what, &r.encode_payload(0).data, &raw.data);
        }
        ClientMessage::Reclaim(r) => {
            *seen.other.entry(raw.msg_type).or_default() += 1;
            assert_same(&what, &r.encode_payload(0).front, &raw.front);
        }
    }
}

fn check_mds_message(m: &Msgr1Message, seen: &mut Seen, snap_format: SnapFormat) {
    let raw = m.raw.clone();
    let what = format!(
        "mds msg seq {} type {:#x} v{} tid {}",
        m.header.seq, raw.msg_type, raw.version, raw.tid
    );
    match raw.msg_type {
        msg_type::CLIENT_SESSION => {
            let s = MClientSession::from_raw(raw.clone()).unwrap_or_else(|e| panic!("{what}: {e}"));
            *seen
                .session_ops
                .entry(format!("m:{}", s.op.name()))
                .or_default() += 1;
            assert!(s.op.is_mds_op(), "{what}");
            let ours = s.to_raw(0);
            assert_eq!(ours.version, raw.version, "{what}: version choice");
            assert_same(&what, &ours.front, &raw.front);
        }
        msg_type::CLIENT_REPLY => {
            let r = MClientReply::from_raw(raw.clone()).unwrap_or_else(|e| panic!("{what}: {e}"));
            *seen.reply_ops.entry(r.op).or_default() += 1;
            assert_eq!(r.tid, raw.tid);
            assert!(
                !matches!(r.extra, ReplyExtra::Raw(_)),
                "{what}: unparsed extra for {:?}",
                r.op
            );
            // Ceph 20.2 sends InodeStat v8.
            assert_same(&what, &r.encode_payload_inodestat_v(8).front, &raw.front);
            if !r.snapbl.is_empty() {
                let t = SnapTrace::decode(r.snapbl.clone(), snap_format)
                    .unwrap_or_else(|e| panic!("{what}: snap trace: {e}"));
                assert!(t.is_well_formed(), "{what}: {t:?}");
                assert_eq!(t.encode(snap_format), r.snapbl);
            }
            let violations = r.check_invariants();
            assert!(violations.is_empty(), "{what} ({:?}): {violations:?}", r.op);
        }
        msg_type::CLIENT_CAPS => {
            let c = MClientCaps::from_raw(raw.clone()).unwrap_or_else(|e| panic!("{what}: {e}"));
            *seen
                .caps_from_mds
                .entry((c.op.name().to_string(), raw.version))
                .or_default() += 1;
            assert!(c.op.is_mds_op(), "{what}");
            let p = c.encode_payload_version(raw.version);
            assert_same(&what, &p.front, &raw.front);
            assert_same(&format!("{what} middle"), &p.middle, &raw.middle);
            if !c.snapbl.is_empty() {
                SnapTrace::decode(c.snapbl.clone(), snap_format)
                    .unwrap_or_else(|e| panic!("{what}: snap trace: {e}"));
            }
            if !c.xattrbl.is_empty() {
                decode_xattr_blob(c.xattrbl.clone())
                    .unwrap_or_else(|e| panic!("{what}: xattrs: {e}"));
            }
        }
        msg_type::CLIENT_SNAP => {
            let s = MClientSnap::from_raw(raw.clone()).unwrap_or_else(|e| panic!("{what}: {e}"));
            *seen.other.entry(raw.msg_type).or_default() += 1;
            assert_same(&what, &s.encode_payload(0).front, &raw.front);
            SnapTrace::decode(s.trace, snap_format)
                .unwrap_or_else(|e| panic!("{what}: snap trace: {e}"));
        }
        msg_type::CLIENT_QUOTA => {
            let q = MClientQuota::from_raw(raw.clone()).unwrap_or_else(|e| panic!("{what}: {e}"));
            *seen.other.entry(raw.msg_type).or_default() += 1;
            assert_same(&what, &q.encode_payload(0).front, &raw.front);
        }
        msg_type::CLIENT_LEASE => {
            let l = MClientLease::from_raw(raw.clone()).unwrap_or_else(|e| panic!("{what}: {e}"));
            *seen.other.entry(raw.msg_type).or_default() += 1;
            assert_same(&what, &l.encode_payload(0).front, &raw.front);
        }
        msg_type::CLIENT_REQUEST_FORWARD => {
            let f = MClientRequestForward::from_raw(raw.clone())
                .unwrap_or_else(|e| panic!("{what}: {e}"));
            *seen.other.entry(raw.msg_type).or_default() += 1;
            assert_same(&what, &f.encode_payload(0).front, &raw.front);
        }
        t => {
            *seen.other.entry(t).or_default() += 1;
        }
    }
}

fn check_capture(name: &str, snap_format: SnapFormat) -> Option<Seen> {
    let convs = mds_conversations(name)?;
    let mut seen = Seen::default();
    for conv in &convs {
        assert!(!conv.truncated, "{name}: a stream was cut off");
        for m in &conv.from_client {
            check_client_message(m, &mut seen);
        }
        for m in &conv.from_server {
            check_mds_message(m, &mut seen, snap_format);
        }
        // Every reply, paired with its request, satisfies the request-aware invariants.
        let requests: BTreeMap<u64, MClientRequest> = conv
            .from_client
            .iter()
            .filter(|m| m.raw.msg_type == msg_type::CLIENT_REQUEST)
            .map(|m| {
                let r = MClientRequest::from_raw(m.raw.clone()).unwrap();
                (r.tid, r)
            })
            .collect();
        for m in conv
            .from_server
            .iter()
            .filter(|m| m.raw.msg_type == msg_type::CLIENT_REPLY)
        {
            let reply = MClientReply::from_raw(m.raw.clone()).unwrap();
            let req = requests
                .get(&reply.tid)
                .unwrap_or_else(|| panic!("{name}: reply tid {} without request", reply.tid));
            let v = reply.check_invariants_for(req);
            assert!(
                v.is_empty(),
                "{name}: reply to {:?} tid {}: {v:?}",
                req.op(),
                req.tid
            );
        }
    }
    eprintln!("{name}: {seen:#?}");
    Some(seen)
}

fn require_ops(name: &str, seen: &Seen, ops: &[MdsOp]) {
    for op in ops {
        assert!(
            seen.request_ops.contains_key(op),
            "{name}: no {op:?} request in the capture"
        );
        assert!(
            seen.reply_ops.contains_key(op),
            "{name}: no {op:?} reply in the capture"
        );
    }
}

#[test]
fn kernel_v1_capture() {
    let Some(seen) = check_capture("kernel-v1", SnapFormat::Old) else {
        return;
    };
    require_ops(
        "kernel-v1",
        &seen,
        &[MdsOp::GETATTR, MdsOp::MKDIR, MdsOp::CREATE, MdsOp::READDIR],
    );
    assert!(
        seen.session_ops.contains_key("c:request_open") && seen.session_ops.contains_key("m:open")
    );
    assert!(
        seen.caps_from_client.keys().any(|(_, v)| *v == 12),
        "kernel cap messages are v12"
    );
}

#[test]
fn kernel_v1_mdsv1_capture() {
    let Some(seen) = check_capture("kernel-v1-mdsv1", SnapFormat::Old) else {
        return;
    };
    require_ops(
        "kernel-v1-mdsv1",
        &seen,
        &[MdsOp::GETATTR, MdsOp::MKDIR, MdsOp::CREATE, MdsOp::READDIR],
    );
}

#[test]
fn fuse_v1_capture() {
    let Some(seen) = check_capture("fuse-v1", SnapFormat::NewV1) else {
        return;
    };
    // ceph-fuse served this run's lookups from its cache; LOOKUP is covered by kernel-v1 and
    // fuse-v2.
    require_ops(
        "fuse-v1",
        &seen,
        &[MdsOp::GETATTR, MdsOp::MKDIR, MdsOp::CREATE, MdsOp::READDIR],
    );
    assert!(
        seen.caps_from_client.keys().any(|(_, v)| *v >= 12),
        "libcephfs cap messages"
    );
}

#[test]
fn fuse_v2_capture() {
    let Some(seen) = check_capture("fuse-v2", SnapFormat::NewV1) else {
        return;
    };
    require_ops(
        "fuse-v2",
        &seen,
        &[
            MdsOp::GETATTR,
            MdsOp::LOOKUP,
            MdsOp::MKDIR,
            MdsOp::CREATE,
            MdsOp::READDIR,
        ],
    );
    assert!(
        seen.caps_from_client.keys().any(|(_, v)| *v >= 12),
        "libcephfs cap messages"
    );
}

/// Print every client<->MDS message of a capture, decoded:
/// `PROTO_DUMP=kernel-v1 cargo test -p proto --test captures dump -- --ignored --nocapture`.
#[test]
#[ignore]
fn dump() {
    let name = std::env::var("PROTO_DUMP").unwrap_or_else(|_| "kernel-v1".into());
    let Some(convs) = mds_conversations(&name) else {
        return;
    };
    for conv in convs {
        let mut all: Vec<(&str, &Msgr1Message)> =
            conv.from_client.iter().map(|m| ("C>M", m)).collect();
        all.extend(conv.from_server.iter().map(|m| ("M>C", m)));
        // Interleave by sequence number per direction is not possible without timestamps; print
        // each direction in order.
        for (dir, m) in all {
            let raw = m.raw.clone();
            let text = match raw.msg_type {
                msg_type::CLIENT_REPLY => format!("{:#?}", MClientReply::from_raw(raw).unwrap()),
                msg_type::CLIENT_CAPS => format!("{:#?}", MClientCaps::from_raw(raw).unwrap()),
                msg_type::CLIENT_SNAP => format!("{:#?}", MClientSnap::from_raw(raw).unwrap()),
                msg_type::CLIENT_QUOTA => format!("{:#?}", MClientQuota::from_raw(raw).unwrap()),
                msg_type::CLIENT_SESSION if dir == "M>C" => {
                    format!("{:#?}", MClientSession::from_raw(raw).unwrap())
                }
                _ => format!("{:#?}", ClientMessage::from_raw(raw).unwrap()),
            };
            println!(
                "==== {dir} seq {} type {:#x} v{}\n{text}",
                m.header.seq, m.header.msg_type, m.header.version
            );
        }
    }
}
