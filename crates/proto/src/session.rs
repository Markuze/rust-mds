//! `MClientSession` (`CEPH_MSG_CLIENT_SESSION`, `src/messages/MClientSession.h`).
//!
//! Front, by header version:
//!
//! | v | field |
//! |---|---|
//! | 1 | raw `ceph_mds_session_head`: `u32 op`, `u64 seq`, `ceph_timespec stamp`, `u32 max_caps`, `u32 max_leases` (28 bytes) |
//! | 2 | `map<string,string> metadata` |
//! | 3 | `feature_bitset_t supported_features` |
//! | 4 | `metric_spec_t metric_spec` |
//! | 5 | `u32 flags` (`SESSION_BLOCKLISTED`) |
//! | 6 | `vector<MDSCapAuth> cap_auths` |
//! | 7 | `u64 oldest_client_tid` (absent = `u64::MAX`, "do not trim") |
//!
//! Clients send `REQUEST_OPEN`/`REQUEST_RENEWCAPS` as v7 (the kernel's other ops carry header
//! version 0, i.e. the head only; every version is decoded). The MDS sends v1 (head only) when
//! nothing beyond the head is set and v7 otherwise, as C++ does; unlike C++, a non-default
//! `flags`, `metric_spec`, `cap_auths` or `oldest_client_tid` also selects v7, so a REJECT always
//! carries its flags (catalog F-PROTO-030).

use std::collections::BTreeMap;
use std::fmt;

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode, Message, Payload, RawMessage, Result, UTime};

use crate::features::{FeatureBitset, MetricSpec, mds_supported_features};
use crate::msg_type;
use crate::types::decode_counted;
use crate::util::{Ctx, check_raw, get, get_string_lossy, get_string_map_lossy};

/// `CEPH_SESSION_*` session ops.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionOp(pub u32);

impl SessionOp {
    pub const REQUEST_OPEN: SessionOp = SessionOp(0);
    pub const OPEN: SessionOp = SessionOp(1);
    pub const REQUEST_CLOSE: SessionOp = SessionOp(2);
    pub const CLOSE: SessionOp = SessionOp(3);
    pub const REQUEST_RENEWCAPS: SessionOp = SessionOp(4);
    pub const RENEWCAPS: SessionOp = SessionOp(5);
    pub const STALE: SessionOp = SessionOp(6);
    pub const RECALL_STATE: SessionOp = SessionOp(7);
    pub const FLUSHMSG: SessionOp = SessionOp(8);
    pub const FLUSHMSG_ACK: SessionOp = SessionOp(9);
    pub const FORCE_RO: SessionOp = SessionOp(10);
    pub const REJECT: SessionOp = SessionOp(11);
    pub const REQUEST_FLUSH_MDLOG: SessionOp = SessionOp(12);

    /// `ceph_session_op_name`.
    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "request_open",
            1 => "open",
            2 => "request_close",
            3 => "close",
            4 => "request_renewcaps",
            5 => "renewcaps",
            6 => "stale",
            7 => "recall_state",
            8 => "flushmsg",
            9 => "flushmsg_ack",
            10 => "force_ro",
            11 => "reject",
            12 => "request_flush_mdlog",
            _ => "???",
        }
    }

    /// The ops a client may send. The C++ MDS answers any other op from a client with REJECT,
    /// then evicts and blocklists the client.
    pub fn is_client_op(self) -> bool {
        matches!(self.0, 0 | 2 | 4 | 9 | 12)
    }

    /// The eight ops libcephfs handles; it `ceph_abort()`s on any other op from the MDS.
    pub fn is_mds_op(self) -> bool {
        matches!(self.0, 1 | 3 | 5 | 6 | 7 | 8 | 10 | 11)
    }
}

impl fmt::Debug for SessionOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// `MClientSession::SESSION_BLOCKLISTED`: REJECT flag telling the client it is blocklisted
/// (the kernel's `recover_session=clean` relies on it).
pub const SESSION_BLOCKLISTED: u32 = 1 << 0;

/// The metadata key of the reason carried by REJECT (and CLOSE for `refuse_client_session`).
pub const ERROR_STRING_KEY: &str = "error_string";

/// The C++ blocklist reason; the kernel scans v3/v4 messages for the `blacklisted` substring.
pub const BLOCKLISTED_ERROR_STRING: &str = "blocklisted (blacklisted)";

/// `MDS_AUTH_UID_ANY`.
pub const MDS_AUTH_UID_ANY: i64 = -1;

/// `MDSCapMatch` (`src/mds/MDSAuthCaps.h`): `ENCODE_START(1, 1)` { `i64 uid`, `vector<u32>
/// gids`, `string path`, `string fs_name`, `bool root_squash` }. The kernel skips the 6-byte
/// header and reads the fields by position, so the layout must not grow.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MdsCapMatch {
    /// [`MDS_AUTH_UID_ANY`] for any uid.
    pub uid: i64,
    pub gids: Vec<u32>,
    /// Required path prefix ("" or "/" for any).
    pub path: String,
    /// File system name ("" or "*" for any).
    pub fs_name: String,
    pub root_squash: bool,
}

impl Default for MdsCapMatch {
    fn default() -> Self {
        MdsCapMatch {
            uid: MDS_AUTH_UID_ANY,
            gids: Vec::new(),
            path: String::new(),
            fs_name: String::new(),
            root_squash: false,
        }
    }
}

impl Encode for MdsCapMatch {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        denc::encode_versioned(buf, 1, 1, |b| {
            b.put_i64_le(self.uid);
            self.gids.encode(b, f);
            self.path.encode(b, f);
            self.fs_name.encode(b, f);
            self.root_squash.encode(b, f);
        });
    }
}

impl Decode for MdsCapMatch {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut fr = denc::decode_start(buf, 1, "MDSCapMatch")?;
        let b = &mut fr.body;
        Ok(MdsCapMatch {
            uid: get(b, "MDSCapMatch.uid")?,
            gids: get(b, "MDSCapMatch.gids")?,
            path: get_string_lossy(b, "MDSCapMatch.path")?,
            fs_name: get_string_lossy(b, "MDSCapMatch.fs_name")?,
            root_squash: get(b, "MDSCapMatch.root_squash")?,
        })
    }
}

/// `MDSCapAuth`: `ENCODE_START(1, 1)` { `MDSCapMatch`, `bool readable`, `bool writeable` }. One
/// per grant of the session's MDS caps; clients enforce them locally.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct MdsCapAuth {
    pub cap_match: MdsCapMatch,
    pub readable: bool,
    pub writeable: bool,
}

impl Encode for MdsCapAuth {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        denc::encode_versioned(buf, 1, 1, |b| {
            self.cap_match.encode(b, f);
            self.readable.encode(b, f);
            self.writeable.encode(b, f);
        });
    }
}

impl Decode for MdsCapAuth {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut fr = denc::decode_start(buf, 1, "MDSCapAuth")?;
        let b = &mut fr.body;
        Ok(MdsCapAuth {
            cap_match: MdsCapMatch::decode(b)?,
            readable: get(b, "MDSCapAuth.readable")?,
            writeable: get(b, "MDSCapAuth.writeable")?,
        })
    }
}

/// `MClientSession`. See the module documentation for the wire format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MClientSession {
    pub op: SessionOp,
    /// REQUEST_OPEN/REQUEST_CLOSE: the client's count of `push_seq` messages; RENEWCAPS: echo of
    /// the request's seq; FLUSHMSG(_ACK): the flush seq; OPEN: 0, or `push_seq` for the
    /// duplicate-open answer.
    pub seq: u64,
    /// Read by no peer; the kernel leaves it uninitialized.
    pub stamp: UTime,
    /// RECALL_STATE: the new cap limit.
    pub max_caps: u32,
    pub max_leases: u32,
    /// Client metadata (REQUEST_OPEN: `hostname`, `root`, `entity_id`, `kernel_version` or
    /// `ceph_version`, `uuid`, `timeout`, ...); `error_string` on REJECT.
    pub metadata: BTreeMap<String, String>,
    /// The sender's CephFS features ([`crate::features`]).
    pub supported_features: FeatureBitset,
    pub metric_spec: MetricSpec,
    /// [`SESSION_BLOCKLISTED`].
    pub flags: u32,
    /// OPEN: the session's MDS cap grants, enforced by the clients.
    pub cap_auths: Vec<MdsCapAuth>,
    /// REQUEST_OPEN/RENEWCAPS (v7): lowest client tid still pending; `u64::MAX` = not sent.
    pub oldest_client_tid: u64,
}

impl MClientSession {
    /// A session message with only the head set.
    pub fn new(op: SessionOp, seq: u64) -> Self {
        MClientSession {
            op,
            seq,
            stamp: UTime::default(),
            max_caps: 0,
            max_leases: 0,
            metadata: BTreeMap::new(),
            supported_features: FeatureBitset::new(),
            metric_spec: MetricSpec::default(),
            flags: 0,
            cap_auths: Vec::new(),
            oldest_client_tid: u64::MAX,
        }
    }

    /// `OPEN`: the open acknowledgement (seq 0), the reconnect success (seq 0) or the answer to
    /// a duplicate REQUEST_OPEN (seq = `push_seq`). Carries the MDS feature bitmap, the metric
    /// spec and the session's cap grants (v7). Unlike C++, the duplicate-open answer should also
    /// carry the metric spec (catalog F-PROTO-037), which this constructor always sets.
    pub fn open(
        seq: u64,
        supported_features: FeatureBitset,
        metric_spec: MetricSpec,
        cap_auths: Vec<MdsCapAuth>,
    ) -> Self {
        MClientSession { supported_features, metric_spec, cap_auths, ..Self::new(SessionOp::OPEN, seq) }
    }

    /// `OPEN` with this MDS's advertised features and metric spec
    /// ([`crate::features::MDS_SUPPORTED_FEATURES`], [`crate::features::MDS_METRIC_SPEC`]).
    pub fn open_default(seq: u64, cap_auths: Vec<MdsCapAuth>) -> Self {
        Self::open(seq, mds_supported_features(), MetricSpec::mds_default(), cap_auths)
    }

    /// `RENEWCAPS` echoing the client's `REQUEST_RENEWCAPS` seq.
    pub fn renewcaps(seq: u64) -> Self {
        Self::new(SessionOp::RENEWCAPS, seq)
    }

    /// `CLOSE`: close acknowledgement, or "reconnect denied" during reconnect.
    pub fn close() -> Self {
        Self::new(SessionOp::CLOSE, 0)
    }

    /// `CLOSE` with a reason (C++ adds one only for `refuse_client_session`).
    pub fn close_with_reason(reason: &str) -> Self {
        let mut m = Self::close();
        m.metadata.insert(ERROR_STRING_KEY.to_owned(), reason.to_owned());
        m
    }

    /// `STALE`: the session's caps are void until it renews.
    pub fn stale() -> Self {
        Self::new(SessionOp::STALE, 0)
    }

    /// `RECALL_STATE`: trim caps down to `max_caps`.
    pub fn recall_state(max_caps: u32) -> Self {
        MClientSession { max_caps, ..Self::new(SessionOp::RECALL_STATE, 0) }
    }

    /// `FLUSHMSG` with the current `push_seq`; the client flushes its cap releases and answers
    /// `FLUSHMSG_ACK` with the same seq.
    pub fn flushmsg(seq: u64) -> Self {
        Self::new(SessionOp::FLUSHMSG, seq)
    }

    /// `FORCE_RO`: the metadata cache is read-only.
    pub fn force_ro() -> Self {
        Self::new(SessionOp::FORCE_RO, 0)
    }

    /// `REJECT` with a reason (always v7, so the flags reach the kernel). Send it only in answer
    /// to REQUEST_OPEN or a reconnect: the kernel WARNs on any other REJECT.
    pub fn reject(error_string: &str, blocklisted: bool) -> Self {
        let mut m = Self::new(SessionOp::REJECT, 0);
        m.metadata.insert(ERROR_STRING_KEY.to_owned(), error_string.to_owned());
        if blocklisted {
            m.flags |= SESSION_BLOCKLISTED;
        }
        m
    }

    /// `REJECT` of a blocklisted client, with the C++ reason string and the flag.
    pub fn reject_blocklisted() -> Self {
        Self::reject(BLOCKLISTED_ERROR_STRING, true)
    }

    pub fn error_string(&self) -> Option<&str> {
        self.metadata.get(ERROR_STRING_KEY).map(String::as_str)
    }

    pub fn is_blocklisted(&self) -> bool {
        self.flags & SESSION_BLOCKLISTED != 0
    }

    /// The header version [`MClientSession::to_raw`] uses: 1 when only the head is set, else 7.
    pub fn encode_version(&self) -> u16 {
        let head_only = self.metadata.is_empty()
            && self.supported_features.is_empty()
            && self.metric_spec.is_empty()
            && self.flags == 0
            && self.cap_auths.is_empty()
            && self.oldest_client_tid == u64::MAX;
        if head_only { 1 } else { 7 }
    }

    fn encode_head(&self, b: &mut BytesMut) {
        b.put_u32_le(self.op.0);
        b.put_u64_le(self.seq);
        self.stamp.encode(b, 0);
        b.put_u32_le(self.max_caps);
        b.put_u32_le(self.max_leases);
    }
}

impl Message for MClientSession {
    const TYPE: u16 = msg_type::CLIENT_SESSION;
    const HEAD_VERSION: u16 = 7;
    const COMPAT_VERSION: u16 = 1;

    /// Encodes v1 or v7 (see [`MClientSession::encode_version`]); use
    /// [`MClientSession::to_raw`], which labels the header accordingly.
    fn encode_payload(&self, features: u64) -> Payload {
        let mut b = BytesMut::with_capacity(64);
        self.encode_head(&mut b);
        if self.encode_version() == 7 {
            self.metadata.encode(&mut b, features);
            self.supported_features.encode(&mut b, features);
            self.metric_spec.encode(&mut b, features);
            b.put_u32_le(self.flags);
            b.put_u32_le(self.cap_auths.len() as u32);
            for c in &self.cap_auths {
                c.encode(&mut b, features);
            }
            b.put_u64_le(self.oldest_client_tid);
        }
        Payload { front: b, ..Default::default() }
    }

    fn decode_payload(version: u16, mut front: Bytes, _middle: Bytes, _data: Bytes) -> Result<Self> {
        let p = &mut front;
        denc::need(p, 28, "ceph_mds_session_head")?;
        let mut m = MClientSession::new(SessionOp(p.get_u32_le()), p.get_u64_le());
        m.stamp = UTime { sec: p.get_u32_le(), nsec: p.get_u32_le() };
        m.max_caps = p.get_u32_le();
        m.max_leases = p.get_u32_le();
        if version >= 2 {
            m.metadata = get_string_map_lossy(p, "MClientSession.metadata")?;
        }
        if version >= 3 {
            m.supported_features = FeatureBitset::decode(p).ctx("MClientSession.supported_features")?;
        }
        if version >= 4 {
            m.metric_spec = MetricSpec::decode(p).ctx("MClientSession.metric_spec")?;
        }
        if version >= 5 {
            m.flags = get(p, "MClientSession.flags")?;
        }
        if version >= 6 {
            let n = get::<u32>(p, "MClientSession.cap_auths")? as usize;
            m.cap_auths = decode_counted(p, n, "MClientSession.cap_auths")?;
        }
        if version >= 7 {
            m.oldest_client_tid = get(p, "MClientSession.oldest_client_tid")?;
        }
        // Trailing bytes (fields of a newer version) are ignored, as C++ does.
        Ok(m)
    }

    fn to_raw(&self, features: u64) -> RawMessage {
        let p = self.encode_payload(features);
        let mut raw = RawMessage::new(Self::TYPE, self.encode_version(), Self::COMPAT_VERSION, p.front.freeze());
        raw.priority = Self::PRIORITY;
        raw
    }

    fn from_raw(raw: RawMessage) -> Result<Self> {
        check_raw::<Self>(&raw)?;
        Self::decode_payload(raw.version, raw.front, raw.middle, raw.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{KERNEL_7_0_FEATURES, MDS_METRIC_SPEC, MDS_SUPPORTED_FEATURES};

    #[test]
    fn head_only_ops_are_v1() {
        for m in [
            MClientSession::renewcaps(7),
            MClientSession::close(),
            MClientSession::stale(),
            MClientSession::recall_state(100),
            MClientSession::flushmsg(3),
            MClientSession::force_ro(),
        ] {
            assert!(m.op.is_mds_op());
            let raw = m.to_raw(0);
            assert_eq!(raw.version, 1, "{:?}", m.op);
            assert_eq!(raw.front.len(), 28);
            assert_eq!(MClientSession::from_raw(raw).unwrap(), m);
        }
        let raw = MClientSession::recall_state(100).to_raw(0);
        assert_eq!(
            hex::encode(&raw.front),
            "07000000".to_owned() + "0000000000000000" + "0000000000000000" + "64000000" + "00000000"
        );
        let raw = MClientSession::renewcaps(7).to_raw(0);
        assert_eq!(&raw.front[4..12], &7u64.to_le_bytes());
    }

    #[test]
    fn open_is_v7_with_kernel_readable_layout() {
        let cap = MdsCapAuth {
            cap_match: MdsCapMatch { uid: -1, gids: vec![], path: "/".into(), fs_name: "a".into(), root_squash: false },
            readable: true,
            writeable: true,
        };
        let m = MClientSession::open_default(0, vec![cap.clone()]);
        let raw = m.to_raw(0);
        assert_eq!((raw.version, raw.compat_version), (7, 1));
        let f = &raw.front[..];
        let mut off = 28;
        assert_eq!(&f[off..off + 4], &[0, 0, 0, 0]); // empty metadata
        off += 4;
        // The kernel reads exactly the first 8 bytes of the bitmap.
        assert_eq!(&f[off..off + 4], &8u32.to_le_bytes());
        assert_eq!(&f[off + 4..off + 12], &MDS_SUPPORTED_FEATURES.to_le_bytes());
        off += 12;
        // metric spec: ENCODE_START(1,1), len 12, bitmap
        assert_eq!(&f[off..off + 6], &[1, 1, 12, 0, 0, 0]);
        assert_eq!(&f[off + 10..off + 18], &MDS_METRIC_SPEC.to_le_bytes());
        off += 18;
        assert_eq!(&f[off..off + 4], &[0, 0, 0, 0]); // flags
        off += 4;
        assert_eq!(&f[off..off + 4], &1u32.to_le_bytes()); // one cap auth
        off += 4;
        // MDSCapAuth header, MDSCapMatch header, then the fields the kernel reads positionally.
        assert_eq!(&f[off..off + 2], &[1, 1]);
        assert_eq!(&f[off + 6..off + 8], &[1, 1]);
        let mut p = off + 12;
        assert_eq!(&f[p..p + 8], &(-1i64).to_le_bytes());
        p += 8;
        assert_eq!(&f[p..p + 4], &[0, 0, 0, 0]); // gids
        p += 4;
        assert_eq!(&f[p..p + 5], &[1, 0, 0, 0, b'/']);
        p += 5;
        assert_eq!(&f[p..p + 5], &[1, 0, 0, 0, b'a']);
        p += 5;
        assert_eq!(&f[p..p + 3], &[0, 1, 1]); // root_squash, readable, writeable
        p += 3;
        assert_eq!(&f[p..], &u64::MAX.to_le_bytes()); // oldest_client_tid
        let back = MClientSession::from_raw(raw).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.cap_auths, vec![cap]);
    }

    #[test]
    fn reject_carries_reason_and_flag() {
        let m = MClientSession::reject_blocklisted();
        let raw = m.to_raw(0);
        assert_eq!(raw.version, 7);
        let back = MClientSession::from_raw(raw).unwrap();
        assert_eq!(back.error_string(), Some("blocklisted (blacklisted)"));
        assert!(back.is_blocklisted());
        assert_eq!(back.op, SessionOp::REJECT);
        // Even without a reason the flag forces v7.
        let mut m = MClientSession::new(SessionOp::REJECT, 0);
        m.flags = SESSION_BLOCKLISTED;
        assert_eq!(m.encode_version(), 7);
    }

    /// The kernel's REQUEST_OPEN as described by the map (kclient 3.4.2): v7 with four metadata
    /// keys, an 8-byte feature bitmap, the metric spec, flags 0, no cap auths, oldest tid.
    #[test]
    fn decode_kernel_request_open() {
        let mut b = BytesMut::new();
        b.put_u32_le(0);
        b.put_u64_le(0);
        b.put_slice(&[0xaa; 16]); // stamp, max_caps, max_leases: uninitialized in the kernel
        let mut md = BTreeMap::new();
        for (k, v) in [("hostname", "node1"), ("kernel_version", "7.0.0"), ("entity_id", "admin"), ("root", "/")] {
            md.insert(k.to_string(), v.to_string());
        }
        md.encode(&mut b, 0);
        FeatureBitset::from_word(KERNEL_7_0_FEATURES).encode(&mut b, 0);
        b.put_slice(&hex::decode("01010c00000008000000ffff010000000000").unwrap());
        b.put_u32_le(0);
        b.put_u32_le(0);
        b.put_u64_le(42);
        let raw = RawMessage::new(msg_type::CLIENT_SESSION, 7, 1, b.freeze());
        let m = MClientSession::from_raw(raw).unwrap();
        assert_eq!(m.op, SessionOp::REQUEST_OPEN);
        assert!(m.op.is_client_op());
        assert_eq!(m.metadata["root"], "/");
        assert_eq!(m.supported_features.first_word(), KERNEL_7_0_FEATURES);
        assert!(m.metric_spec.metric_flags.test(16));
        assert_eq!(m.oldest_client_tid, 42);
        assert_eq!(m.stamp, UTime { sec: 0xaaaa_aaaa, nsec: 0xaaaa_aaaa });
    }

    #[test]
    fn decode_old_versions_and_version_zero() {
        // Version 0 (kernel REQUEST_CLOSE etc.): the head only, even with extra bytes present.
        let mut b = BytesMut::new();
        MClientSession::new(SessionOp::REQUEST_CLOSE, 9).encode_head(&mut b);
        b.put_slice(&[1, 2, 3]);
        let raw = RawMessage::new(msg_type::CLIENT_SESSION, 0, 0, b.freeze());
        let m = MClientSession::from_raw(raw).unwrap();
        assert_eq!((m.op, m.seq, m.oldest_client_tid), (SessionOp::REQUEST_CLOSE, 9, u64::MAX));
        // A v4 open (CentOS Stream 9 kernels) decodes with the later fields defaulted.
        let mut b = BytesMut::new();
        MClientSession::new(SessionOp::REQUEST_OPEN, 0).encode_head(&mut b);
        BTreeMap::<String, String>::new().encode(&mut b, 0);
        FeatureBitset::from_word(0x3ff).encode(&mut b, 0);
        MetricSpec::from_word(0xff).encode(&mut b, 0);
        let m = MClientSession::from_raw(RawMessage::new(msg_type::CLIENT_SESSION, 4, 1, b.freeze())).unwrap();
        assert_eq!(m.supported_features.first_word(), 0x3ff);
        assert_eq!(m.flags, 0);
        assert_eq!(m.oldest_client_tid, u64::MAX);
        // Truncated v7 fails cleanly.
        let raw = MClientSession::open_default(0, vec![]).to_raw(0);
        let short = RawMessage::new(msg_type::CLIENT_SESSION, 7, 1, raw.front.slice(..raw.front.len() - 1));
        assert!(MClientSession::from_raw(short).is_err());
        // A newer compat version is refused.
        let newer = RawMessage::new(msg_type::CLIENT_SESSION, 9, 8, raw.front.clone());
        assert!(MClientSession::from_raw(newer).is_err());
    }

    #[test]
    fn op_sets() {
        let mds_ops: Vec<u32> = (0..14).filter(|o| SessionOp(*o).is_mds_op()).collect();
        assert_eq!(mds_ops, vec![1, 3, 5, 6, 7, 8, 10, 11]);
        let client_ops: Vec<u32> = (0..14).filter(|o| SessionOp(*o).is_client_op()).collect();
        assert_eq!(client_ops, vec![0, 2, 4, 9, 12]);
        assert_eq!(format!("{:?}", SessionOp::REQUEST_FLUSH_MDLOG), "request_flush_mdlog");
    }
}
