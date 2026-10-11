//! Cross-checks against the C++ encoders and decoders through `ceph-dencoder` (Ceph 20.2 in
//! the dev cluster). Each test is skipped, with a note on stderr, when the tool is missing.
//!
//! Messages travel in the `encode_message` container ([`proto::wire::encode_message`]). The C++
//! decoder must accept them (several C++ decoders assert on trailing or missing bytes) and its
//! `print()` summary must show the field values we encoded. Re-encoding by `ceph-dencoder` is an
//! identity copy for messages whose payload is in the front segment (`Message::encode` reuses a
//! decoded payload), so byte comparisons are made only where C++ really re-encodes: the plain
//! types (`openc_response_t`, `feature_bitset_t`, ...) and `MClientReconnect` (data segment).

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::{BufMut, Bytes, BytesMut};
use denc::{Decode, Encode, EntityName, EntityType, Message, RawMessage, UTime};
use proto::request::{OpenArgs, ReaddirArgs, SetattrArgs, request_flags};
use proto::*;

fn dencoder() -> Option<&'static str> {
    static FOUND: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let ok = *FOUND.get_or_init(|| {
        Command::new("ceph-dencoder").arg("version").output().map(|o| o.status.success()).unwrap_or(false)
    });
    if ok { Some("ceph-dencoder") } else { None }
}

macro_rules! need_dencoder {
    () => {
        match dencoder() {
            Some(d) => d,
            None => {
                eprintln!("ceph-dencoder not found; skipping");
                return;
            }
        }
    };
}

fn tmp(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("denc-{}-{n}-{name}", std::process::id()))
}

fn run(args: &[&str]) -> Result<String, String> {
    let out = Command::new(dencoder().unwrap()).args(args).output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    if !out.status.success() || stderr.contains("error") || stdout.contains("error") {
        return Err(format!("ceph-dencoder {args:?} failed: {stdout}{stderr}"));
    }
    Ok(stdout)
}

/// `ceph-dencoder type <ty> import <bytes> decode dump_json`.
fn decode_json(ty: &str, bytes: &[u8]) -> String {
    let f = tmp("in");
    std::fs::write(&f, bytes).unwrap();
    run(&["type", ty, "import", f.to_str().unwrap(), "decode", "dump_json"]).unwrap()
}

/// `ceph-dencoder type <ty> import <bytes> decode encode export <out>`.
fn reencode(ty: &str, bytes: &[u8]) -> Vec<u8> {
    let f = tmp("in");
    let o = tmp("out");
    std::fs::write(&f, bytes).unwrap();
    run(&["type", ty, "import", f.to_str().unwrap(), "decode", "encode", "export", o.to_str().unwrap()]).unwrap();
    std::fs::read(o).unwrap()
}

/// The C++ `generate_test_instances` object `n`, encoded.
fn test_instance(ty: &str, n: u32) -> Vec<u8> {
    let o = tmp("ti");
    run(&["type", ty, "select_test", &n.to_string(), "encode", "export", o.to_str().unwrap()]).unwrap();
    std::fs::read(o).unwrap()
}

fn with_src(mut raw: RawMessage, ty: EntityType, num: i64) -> RawMessage {
    raw.src = Some(EntityName::new(ty, num));
    raw
}

/// Our message through the C++ decoder and encoder: returns (summary json, C++ re-encoding).
/// The re-encoding is a real one only for messages without a front payload (reconnect);
/// otherwise `Message::encode` reuses the decoded payload.
fn cpp_roundtrip(ty: &str, raw: &RawMessage) -> (String, RawMessage) {
    let bytes = wire::encode_message(raw);
    let json = decode_json(ty, &bytes);
    let back = wire::decode_message(&reencode(ty, &bytes)).unwrap();
    (json, back)
}

fn summary(json: &str) -> String {
    json.lines().find(|l| l.contains("\"summary\"")).unwrap_or(json).trim().to_owned()
}

#[test]
fn session_messages() {
    need_dencoder!();
    let cap = MdsCapAuth {
        cap_match: MdsCapMatch {
            uid: -1,
            gids: vec![1, 2],
            path: "/volumes".into(),
            fs_name: "a".into(),
            root_squash: true,
        },
        readable: true,
        writeable: false,
    };
    for m in [
        MClientSession::open_default(0, vec![cap.clone()]),
        MClientSession::open_default(7, vec![]),
        MClientSession::reject_blocklisted(),
        MClientSession::reject("missing required features '0x1'", false),
        MClientSession::renewcaps(12),
        MClientSession::recall_state(5000),
        MClientSession::flushmsg(3),
        MClientSession::stale(),
        MClientSession::force_ro(),
        MClientSession::close(),
    ] {
        let raw = with_src(m.to_raw(0), EntityType::MDS, 0);
        let (json, _) = cpp_roundtrip("MClientSession", &raw);
        let s = summary(&json);
        assert!(s.contains(&format!("client_session({}", m.op.name())), "{s}");
        if m.seq != 0 {
            assert!(s.contains(&format!(" seq {}", m.seq)), "{s}");
        }
        if m.op == SessionOp::RECALL_STATE {
            assert!(s.contains("max_caps 5000 max_leases 0"), "{s}");
        }
        if !m.cap_auths.is_empty() {
            assert!(s.contains("cap_auths") && s.contains("/volumes"), "{s}");
        }
    }
    // A client REQUEST_OPEN v7 with metadata, features, metric spec and oldest_client_tid.
    let mut open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
    open.metadata.insert("hostname".into(), "node1".into());
    open.metadata.insert("root".into(), "/".into());
    open.supported_features = FeatureBitset::from_word(features::KERNEL_7_0_FEATURES);
    open.metric_spec = MetricSpec::from_word(0x1ffff);
    open.oldest_client_tid = 4;
    let raw = with_src(open.to_raw(0), EntityType::CLIENT, 4242);
    let (json, _) = cpp_roundtrip("MClientSession", &raw);
    assert!(summary(&json).contains("client_session(request_open)"), "{json}");
}

fn sample_request() -> MClientRequest {
    let mut m = MClientRequest::new(0x1234, MdsOp::CREATE);
    m.head.oldest_client_tid = 0x1230;
    m.head.mdsmap_epoch = 9;
    m.head.flags = request_flags::WANT_DENTRY;
    m.head.num_retry = 1;
    m.head.num_fwd = 2;
    m.head.caller_uid = 1000;
    m.head.caller_gid = 1001;
    m.head.owner_uid = 1000;
    m.head.owner_gid = 1001;
    m.head.ino = 0x100_0000_0123;
    m.head.args = OpenArgs {
        flags: 0o301,
        mode: 0o100644,
        stripe_unit: 0,
        stripe_count: 0,
        object_size: 0,
        pool: -1,
        mask: 1,
        old_size: 0,
    }
    .to_args();
    m.path = FilePath::new(1, Bytes::from_static(b"newfile"));
    m.releases.push(RequestRelease {
        ino: 1,
        cap_id: 7,
        caps: 0x155,
        wanted: 0,
        seq: 3,
        issue_seq: 2,
        mseq: 0,
        dname_seq: 1,
        dname: Bytes::from_static(b"newfile"),
    });
    m.stamp = UTime { sec: 1_760_000_000, nsec: 5 };
    m.gid_list = vec![1001, 27];
    m
}

#[test]
fn request_decoded_by_cpp() {
    need_dencoder!();
    let m = sample_request();
    let raw = with_src(m.to_raw(0), EntityType::CLIENT, 4242);
    let (json, _) = cpp_roundtrip("MClientRequest", &raw);
    let s = summary(&json);
    for want in [
        "client.4242:4660 create",
        "owner_uid=1000, owner_gid=1001",
        "#0x1/newfile",
        "2025-10-09T08:53:20.000000+0000",
        "FWD=2",
        "RETRY=1",
        "caller_uid=1000, caller_gid=1001{1001,27,}",
    ] {
        assert!(s.contains(want), "{want} not in {s}");
    }
    // GETATTR mask and SETFILELOCK args.
    let mut ga = MClientRequest::new(5, MdsOp::GETATTR);
    ga.head.args = RequestArgs::with_getattr_mask(cap::STAT_CAP_INODE_ALL);
    ga.path = FilePath::new(0x100_0000_0000, Bytes::new());
    let s =
        summary(&decode_json("MClientRequest", &wire::encode_message(&with_src(ga.to_raw(0), EntityType::CLIENT, 1))));
    assert!(s.contains("client.1:5 getattr pAsLsXsFs #0x10000000000"), "{s}");
    let mut fl = MClientRequest::new(6, MdsOp::SETFILELOCK);
    fl.head.args =
        proto::request::FilelockArgs { rule: 1, lock_type: 2, owner: 3, pid: 4, start: 5, length: 6, wait: 1 }
            .to_args();
    let s =
        summary(&decode_json("MClientRequest", &wire::encode_message(&with_src(fl.to_raw(0), EntityType::CLIENT, 1))));
    assert!(s.contains("setfilelock rule 1, type 2, owner 3, pid 4, start 5, length 6, wait 1"), "{s}");

    // SETATTR args as the C++ print shows them.
    let mut sa = MClientRequest::new(5, MdsOp::SETATTR);
    sa.head.args = SetattrArgs {
        mode: 0o600,
        uid: 7,
        gid: 8,
        size: 4096,
        mask: setattr::MODE | setattr::UID | setattr::GID | setattr::SIZE,
        ..Default::default()
    }
    .to_args();
    sa.path = FilePath::new(0x100_0000_0000, Bytes::new());
    let s =
        summary(&decode_json("MClientRequest", &wire::encode_message(&with_src(sa.to_raw(0), EntityType::CLIENT, 1))));
    assert!(s.contains("setattr mode=0600 uid=7 gid=8 size=4096"), "{s}");
    // READDIR with a resume name.
    let mut rd = MClientRequest::new(6, MdsOp::READDIR);
    rd.head.args = ReaddirArgs { frag: 0, max_entries: 1024, max_bytes: 512 << 10, flags: 1, offset_hash: 0 }.to_args();
    rd.path = FilePath::new(1, Bytes::new());
    rd.path2 = FilePath::new(0, Bytes::from_static(b"last"));
    let s =
        summary(&decode_json("MClientRequest", &wire::encode_message(&with_src(rd.to_raw(0), EntityType::CLIENT, 1))));
    assert!(s.contains("readdir #0x1 last"), "{s}");
}

#[test]
fn caps_decoded_by_cpp() {
    need_dencoder!();
    let mut x = XattrMap::new();
    x.insert(Bytes::from_static(b"user.a"), Bytes::from_static(b"1"));
    let mut m = MClientCaps::new(CapOp::GRANT, 0x100_0000_0001, 1, 5, 3, 0x0d55, 0x0d00, 0, 1, 2, 17);
    m.set_inode(&CapInodeFields {
        uid: 1000,
        gid: 1001,
        mode: 0o100644,
        nlink: 1,
        size: 4096,
        max_size: 4 << 20,
        truncate_size: u64::MAX,
        truncate_seq: 1,
        mtime: UTime { sec: 5, nsec: 6 },
        atime: UTime { sec: 7, nsec: 8 },
        ctime: UTime { sec: 9, nsec: 10 },
        btime: UTime { sec: 1, nsec: 2 },
        change_attr: 4,
        time_warp_seq: 1,
        layout: FileLayout { pool_ns: "ns".into(), ..FileLayout::default_for_pool(2) },
        nfiles: -1,
        nsubdirs: -1,
        inline_version: CEPH_INLINE_NONE,
        xattrs: Some((2, xattrs_to_blob(&x))),
        fscrypt_auth: Bytes::new(),
        fscrypt_file: Bytes::new(),
    });
    m.tid = 99;
    m.oldest_flush_tid = 98;
    m.flags = 4;
    let mut imp = m.clone();
    imp.op = CapOp::IMPORT;
    imp.peer = CapPeer { cap_id: 4, issue_seq: 1, mseq: 0, mds: 1, flags: 1 };
    imp.snapbl = SnapTrace::new(vec![SnapRealmInfo::new(1, 0, 1)]).encode(SnapFormat::Old);
    let exp = MClientCaps::export(0x1000, 5, 2, CapPeer { cap_id: 9, issue_seq: 2, mseq: 3, mds: 1, flags: 1 }, 3);
    let ack = MClientCaps::flush_ack(0x1000, 5, 7, 1, 0x1000, 3, 42, 40, 9);
    for msg in [m, imp, exp, ack] {
        let raw = with_src(msg.to_raw(0), EntityType::MDS, 0);
        let (json, _) = cpp_roundtrip("MClientCaps", &raw);
        let s = summary(&json);
        let mut want = format!(
            "client_caps({} ino 0x{:x} {} seq {} issue_seq {}",
            msg.op.name(),
            msg.ino,
            msg.cap_id,
            msg.seq,
            msg.issue_seq
        );
        if msg.tid != 0 {
            want += &format!(" tid {}", msg.tid);
        }
        want += &format!(
            " caps={} dirty={} wanted={} follows {}",
            cap::cap_string(msg.caps),
            cap::cap_string(msg.dirty),
            cap::cap_string(msg.wanted),
            msg.snap_follows
        );
        if msg.migrate_seq != 0 {
            want += &format!(" mseq {}", msg.migrate_seq);
        }
        want += &format!(" size {}/{}", msg.size, msg.max_size);
        if msg.truncate_seq != 0 {
            want += &format!(" ts {}/{}", msg.truncate_seq, msg.truncate_size);
        }
        assert!(s.contains(&want), "{want}\n not in {s}");
        assert!(s.contains(&format!("change_attr {}", msg.change_attr)), "{s}");
        if msg.xattr_version != 0 {
            assert!(s.contains(&format!("xattrs(v={} l={})", msg.xattr_version, msg.xattrbl.len())), "{s}");
        }
    }
}

#[test]
fn reply_decoded_by_cpp() {
    need_dencoder!();
    let mut r = MClientReply::new(77, MdsOp::LOOKUP, -2);
    r.mdsmap_epoch = 3;
    r.trace.dentry = Some(TraceDentry {
        dir: InodeStat { ino: 1, mode: 0o40755, nlink: 1, dir_layout: DirLayout { dir_hash: 2 }, ..Default::default() },
        dirstat: DirStat::root(0),
        name: Bytes::from_static(b"x"),
        lease: LeaseStat::null(),
    });
    r.set_snap_trace(&SnapTrace::new(vec![SnapRealmInfo::new(1, 0, 1)]), SnapFormat::Old);
    let raw = with_src(r.to_raw(0), EntityType::MDS, 0);
    let (json, back) = cpp_roundtrip("MClientReply", &raw);
    assert!(summary(&json).contains("client_reply(???:77 = -2 (2) No such file or directory"), "{json}");
    assert_eq!(back.front, raw.front);
    let mut w = MClientReply::new(78, MdsOp::MKDIR, 0);
    w.trace.target = Some(InodeStat { ino: 0x100_0000_0001, mode: 0o40755, nlink: 1, ..Default::default() });
    let (json, _) = cpp_roundtrip("MClientReply", &with_src(w.to_raw(0), EntityType::MDS, 0));
    assert!(summary(&json).contains("client_reply(???:78 = 0 (0) Success safe)"), "{json}");
}

#[test]
fn small_messages_decoded_by_cpp() {
    need_dencoder!();
    let msgs: Vec<(&str, RawMessage, &str)> = vec![
        (
            "MClientCapRelease",
            MClientCapRelease {
                caps: vec![CapItem { ino: 0x1000, cap_id: 2, migrate_seq: 0, issue_seq: 3 }],
                osd_epoch_barrier: 9,
            }
            .to_raw(0),
            "client_cap_release(1)",
        ),
        (
            "MClientLease",
            MClientLease::revoke(1, 2, Bytes::from_static(b"name"), 7).to_raw(0),
            "client_lease(a=revoke seq 7 mask 1 0x1/name)",
        ),
        (
            "MClientSnap",
            MClientSnap::split(
                0x1000,
                vec![0x1001],
                vec![0x2000],
                SnapTrace::new(vec![SnapRealmInfo::new(0x1000, 1, 2), SnapRealmInfo::new(1, 0, 1)])
                    .encode(SnapFormat::Old),
            )
            .to_raw(0),
            "client_snap(split split=0x1000 tracelen=96)",
        ),
        (
            "MClientQuota",
            MClientQuota {
                ino: 0x1000,
                rctime: UTime { sec: 1, nsec: 2 },
                rbytes: 3,
                rfiles: 4,
                rsubdirs: 5,
                quota: QuotaInfo { max_bytes: 6, max_files: 7 },
            }
            .to_raw(0),
            "client_quota( [0x1000] n(v0 rc1.000000 b3 9=4+5) quota(max_bytes = 6 max_files = 7))",
        ),
        (
            "MClientRequestForward",
            MClientRequestForward::new(99, 1, 2).to_raw(0),
            "client_request_forward(99 to mds.1 num_fwd=2 client_must_resend)",
        ),
    ];
    for (ty, raw, want) in msgs {
        let raw = with_src(raw, EntityType::MDS, 0);
        let (json, _) = cpp_roundtrip(ty, &raw);
        assert!(summary(&json).contains(want), "{ty}: {json}");
    }
}

#[test]
fn reconnect_v3_and_v5_through_cpp() {
    need_dencoder!();
    let locks = FlockState {
        fcntl: vec![FileLock { start: 0, length: 10, client: 0, owner: 1 << 63 | 5, pid: 42, lock_type: 2 }],
        flock: vec![],
    };
    let mut m = MClientReconnect::default();
    m.caps.insert(
        0x100_0000_0001,
        CapReconnect {
            path: Bytes::from_static(b"f"),
            cap_id: 3,
            wanted: 0x55,
            issued: 0x555,
            snaprealm: 1,
            pathbase: 1,
            flockbl: locks.encode(),
            snap_follows: 1,
        },
    );
    m.caps.insert(
        1,
        CapReconnect {
            cap_id: 1,
            issued: 0x155,
            snaprealm: 1,
            flockbl: FlockState::default().encode(),
            ..Default::default()
        },
    );
    m.realms.push(SnaprealmReconnect { ino: 1, seq: 1, parent: 0 });
    for v in [3u16, 5] {
        m.version = v;
        m.more = v == 5;
        let raw = with_src(m.to_raw(0), EntityType::CLIENT, 4242);
        let (json, back) = cpp_roundtrip("MClientReconnect", &raw);
        assert!(summary(&json).contains("client_reconnect(2 caps 1 realms )"), "{json}");
        assert_eq!(back.version, v);
        if v == 3 {
            // The C++ v3 encoder writes realm records only in its v1 branch, so its re-encoding
            // lacks the 24-byte realm our (kernel-like) v3 message carries.
            assert_eq!(back.data, raw.data.slice(..raw.data.len() - 24), "v3");
        } else {
            assert_eq!(back.data, raw.data, "v5");
            assert_eq!(MClientReconnect::from_raw(back).unwrap(), m);
        }
    }
}

/// The C++ test instances of the plain types, decoded by us and re-encoded byte for byte.
#[test]
fn cpp_test_instances() {
    need_dencoder!();
    fn check<T: Decode + Encode + std::fmt::Debug>(ty: &str, count: u32) {
        for n in 1..=count {
            let bytes = Bytes::from(test_instance(ty, n));
            let v: T = denc::decode_exact(bytes.clone(), "test instance").unwrap_or_else(|e| panic!("{ty} #{n}: {e}"));
            assert_eq!(denc::encode_to_bytes(&v, 0), bytes, "{ty} #{n}: {v:?}");
        }
    }
    check::<OpencResponse>("openc_response_t", 2);
    check::<FeatureBitset>("feature_bitset_t", 2);
    check::<MetricSpec>("metric_spec_t", 2);
    check::<QuotaInfo>("quota_info_t", 2);
    check::<FilePath>("filepath", 5);
    check::<Frag>("frag_t", 3);
    check::<FragTree>("fragtree_t", 2);
    check::<CapReconnect>("cap_reconnect_t", 1);
    check::<RequestRelease>("MClientRequest::Release", 2);
    // SnapRealmInfo (old format) is not Decode on its own; go through a one-realm trace.
    for n in 1..=4 {
        let bytes = Bytes::from(test_instance("SnapRealmInfo", n));
        let t = SnapTrace::decode(bytes.clone(), SnapFormat::Old).unwrap();
        assert_eq!(t.realms.len(), 1);
        assert_eq!(t.encode(SnapFormat::Old), bytes, "SnapRealmInfo #{n}");
    }
    // ClientMetricMessage: one item = u32 type + frame; parse it as a one-item MClientMetrics.
    let item = test_instance("ClientMetricMessage", 1);
    let mut front = BytesMut::new();
    front.put_u32_le(1);
    front.put_slice(&item);
    let m = MClientMetrics::decode_payload(1, front.freeze(), Bytes::new(), Bytes::new()).unwrap();
    assert_eq!(m.metrics, vec![ClientMetric::CapInfo { hits: 1, misses: 2, total_caps: 3 }]);
}

/// Our encodings of the plain types, decoded by C++ and dumped as JSON.
#[test]
fn cpp_decodes_our_types() {
    need_dencoder!();
    let j = decode_json(
        "openc_response_t",
        &denc::encode_to_bytes(
            &OpencResponse { created_ino: 0x10000000002, delegated_inos: vec![(0x10000000100, 0x40)] },
            0,
        ),
    );
    assert!(j.contains("\"created_ino\": 1099511627778") && j.contains("[0x10000000100~0x40]"), "{j}");
    let j = decode_json("feature_bitset_t", &denc::encode_to_bytes(&features::mds_supported_features(), 0));
    assert!(j.contains("0x00000000003f7fff"), "{j}");
    let j = decode_json("quota_info_t", &denc::encode_to_bytes(&QuotaInfo { max_bytes: 1 << 30, max_files: 100 }, 0));
    assert!(j.contains("\"max_bytes\": 1073741824") && j.contains("\"max_files\": 100"), "{j}");
    let mut ft = FragTree::new();
    ft.splits.insert(Frag::ROOT, 2);
    let j = decode_json("fragtree_t", &denc::encode_to_bytes(&ft, 0));
    assert!(j.contains("\"frag\": \"*\"") || j.contains("\"*\""), "{j}");
    let j = decode_json("frag_t", &denc::encode_to_bytes(&Frag::new(3, 0x00a0_0000), 0));
    assert!(j.contains("\"value\": 10485760") && j.contains("\"bits\": 3"), "{j}");
    let cr = CapReconnect {
        path: Bytes::from_static(b"dir/f"),
        cap_id: 9,
        wanted: 0x1,
        issued: 0x155,
        snaprealm: 1,
        pathbase: 0x1000,
        flockbl: Bytes::new(),
        snap_follows: 0,
    };
    let j = decode_json("cap_reconnect_t", &denc::encode_to_bytes(&cr, 0));
    assert!(
        j.contains("\"path\": \"dir/f\"")
            && j.contains("\"cap_id\": 9")
            && j.contains("\"path base ino\": 4096")
            && j.contains("pAsLsXsFs"),
        "{j}"
    );
    let t = SnapTrace::new(vec![SnapRealmInfo {
        ino: 0x1000,
        created: 2,
        parent: 1,
        parent_since: 2,
        seq: 5,
        my_snaps: vec![5, 3],
        prior_parent_snaps: vec![1],
        ..Default::default()
    }]);
    let j = decode_json("SnapRealmInfo", &t.encode(SnapFormat::Old));
    assert!(j.contains("\"ino\": 4096") && j.contains("\"seq\": 5") && j.contains("\"parent_since\": 2"), "{j}");
    let j = decode_json("filepath", &denc::encode_to_bytes(&FilePath::new(0x1000, Bytes::from_static(b"a/b")), 0));
    assert!(j.contains("\"base_ino\": 4096") && j.contains("\"relative_path\": \"a/b\""), "{j}");
}
