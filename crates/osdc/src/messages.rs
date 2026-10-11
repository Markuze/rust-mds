//! OSD client messages: `MOSDMap` (41), `MOSDOp` (42), `MOSDOpReply` (43), `MOSDBackoff` (61).
//!
//! `MOSDOp` (`src/messages/MOSDOp.h`), versions chosen from the OSD connection's features like
//! `MOSDOp::encode_payload`: v9 needs `SERVER_SQUID`, v8 needs `RESEND_ON_SPLIT` (older OSDs
//! are not supported). Front segment:
//!
//! ```text
//! spg_t pgid (actual pg, shard)   u32 hash (raw object hash)   u32 osdmap_epoch   u32 flags
//! osd_reqid_t reqid               blkin_trace_info (3 × i64)   [v9: jspan_context: ENCODE_START(1,1) bool valid]
//! u32 client_inc   utime mtime    object_locator_t oloc        string oid
//! u16 num_ops, num_ops × ceph_osd_op (38 bytes each)
//! u64 snapid       u64 snap_seq   vector<u64> snaps            i32 retry_attempt   u64 features
//! ```
//!
//! The data segment is the ops' input data concatenated. `MOSDOpReply` v8
//! (`src/messages/MOSDOpReply.h`): `string oid`, `pg_t pgid`, `i64 flags`, `i32 result`,
//! `eversion bad_replay_version`, `u32 osdmap_epoch`, `u32 num_ops`, ops, `i32 retry_attempt`,
//! `num_ops × i32 rval`, `eversion replay_version`, `u64 user_version`, `bool do_redirect`
//! [+ `request_redirect_t`], blkin trace; the data segment carries the ops' output data.

use std::collections::BTreeMap;

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode, Message, Payload, Result, UTime, Uuid, features};
use denc::{encode_finish, encode_start};

use crate::ops::OsdOp;
use crate::types::{EVersion, HObject, ObjectLocator, OsdReqId, PgId, SpgId, invalid};

/// `CEPH_MSG_OSD_MAP`.
pub const CEPH_MSG_OSD_MAP: u16 = 41;
/// `CEPH_MSG_OSD_OP`.
pub const CEPH_MSG_OSD_OP: u16 = 42;
/// `CEPH_MSG_OSD_OPREPLY`.
pub const CEPH_MSG_OSD_OPREPLY: u16 = 43;
/// `CEPH_MSG_OSD_BACKOFF`.
pub const CEPH_MSG_OSD_BACKOFF: u16 = 61;
/// `CEPH_MSG_PING`.
pub const CEPH_MSG_PING: u16 = 2;

/// `osd_client_op_priority` default.
pub const OSD_CLIENT_OP_PRIORITY: u16 = 63;

/// `MOSDMap`: `uuid fsid`, `map<u32, bufferlist> incremental_maps`, `map<u32, bufferlist>
/// maps`, (v2) `u32 cluster_osdmap_trim_lower_bound`, `u32 newest_map`, (v4) `u32 0` (the
/// removed `gap_removed_snaps`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MOsdMap {
    pub fsid: Uuid,
    pub incremental_maps: BTreeMap<u32, Bytes>,
    pub maps: BTreeMap<u32, Bytes>,
    pub cluster_osdmap_trim_lower_bound: u32,
    pub newest_map: u32,
}

impl MOsdMap {
    /// `get_first()`: the lowest epoch carried (full or incremental), 0 if none.
    pub fn first(&self) -> u32 {
        let a = self.maps.keys().next().copied();
        let b = self.incremental_maps.keys().next().copied();
        match (a, b) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => 0,
        }
    }

    /// `get_last()`.
    pub fn last(&self) -> u32 {
        let a = self.maps.keys().next_back().copied().unwrap_or(0);
        let b = self.incremental_maps.keys().next_back().copied().unwrap_or(0);
        a.max(b)
    }
}

impl Message for MOsdMap {
    const TYPE: u16 = CEPH_MSG_OSD_MAP;
    const HEAD_VERSION: u16 = 4;
    const COMPAT_VERSION: u16 = 3;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        self.fsid.encode(&mut b, f);
        self.incremental_maps.encode(&mut b, f);
        self.maps.encode(&mut b, f);
        b.put_u32_le(self.cluster_osdmap_trim_lower_bound);
        b.put_u32_le(self.newest_map);
        b.put_u32_le(0);
        Payload { front: b, ..Default::default() }
    }

    fn decode_payload(version: u16, front: Bytes, _middle: Bytes, _data: Bytes) -> Result<Self> {
        let mut p = front;
        let fsid = Uuid::decode(&mut p)?;
        let incremental_maps = BTreeMap::<u32, Bytes>::decode(&mut p)?;
        let maps = BTreeMap::<u32, Bytes>::decode(&mut p)?;
        let (mut lb, mut newest) = (0, 0);
        if version >= 2 {
            lb = u32::decode(&mut p)?;
            newest = u32::decode(&mut p)?;
        }
        // v4: gap_removed_snaps (removed in octopus), ignored
        Ok(MOsdMap { fsid, incremental_maps, maps, cluster_osdmap_trim_lower_bound: lb, newest_map: newest })
    }
}

/// A `MOSDOp` as the Objecter builds it (`Objecter::_prepare_osd_op`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MOsdOp {
    /// Header tid.
    pub tid: u64,
    pub client_inc: u32,
    pub osdmap_epoch: u32,
    pub flags: u32,
    pub mtime: UTime,
    pub retry_attempt: i32,
    /// The target object (hash = the full-precision placement hash).
    pub hobj: HObject,
    /// The actual pg (seed already folded by `pg_num`).
    pub pgid: SpgId,
    pub ops: Vec<OsdOp>,
    pub snap_seq: u64,
    pub snaps: Vec<u64>,
    /// Feature bits recorded in the message (`CEPH_FEATURES_SUPPORTED_DEFAULT` from Objecter).
    pub features: u64,
    /// The reqid field: the Objecter sends no name and tid 0 with `inc = client_inc` (as
    /// `MOSDOp`'s constructor does), so the OSD uses (message source, inc, header tid).
    pub reqid: OsdReqId,
    /// Message priority (`osd_client_op_priority` unless set).
    pub priority: u16,
}

impl MOsdOp {
    /// `object_locator_t` the message carries: from the hobject, or (pg ops, empty name) pool,
    /// namespace and hash.
    pub fn object_locator(&self) -> ObjectLocator {
        if self.hobj.oid.is_empty() {
            ObjectLocator { pool: self.hobj.pool, key: Bytes::new(), nspace: self.hobj.nspace.clone(), hash: i64::from(self.hobj.hash) }
        } else {
            ObjectLocator { pool: self.hobj.pool, key: self.hobj.key.clone(), nspace: self.hobj.nspace.clone(), hash: -1 }
        }
    }

    /// Header version for a peer with `features`.
    pub fn version_for(features: u64) -> Option<u16> {
        if features::has(features, features::MASK_SERVER_SQUID) {
            Some(9)
        } else if features::has(features, features::MASK_RESEND_ON_SPLIT) {
            Some(8)
        } else {
            None
        }
    }

    /// Encode for an OSD connection with `features` (v8 or v9). Returns the raw message, or
    /// `None` for pre-luminous OSDs.
    pub fn to_raw_for(&self, peer_features: u64) -> Option<denc::RawMessage> {
        let v = Self::version_for(peer_features)?;
        let mut b = BytesMut::new();
        self.pgid.encode(&mut b, peer_features);
        b.put_u32_le(self.hobj.hash);
        b.put_u32_le(self.osdmap_epoch);
        b.put_u32_le(self.flags);
        self.reqid.encode(&mut b, peer_features);
        // blkin_trace_info { trace_id, span_id, parent_span_id } (no tracing)
        b.put_bytes(0, 24);
        if v >= 9 {
            // jspan_context: ENCODE_START(1, 1) bool is_valid = false
            let s = encode_start(&mut b, 1, 1);
            b.put_u8(0);
            encode_finish(&mut b, s);
        }
        b.put_u32_le(self.client_inc);
        self.mtime.encode(&mut b, peer_features);
        self.object_locator().encode(&mut b, peer_features);
        self.hobj.oid.encode(&mut b, peer_features);
        b.put_u16_le(self.ops.len() as u16);
        let mut data = BytesMut::new();
        for o in &self.ops {
            // merge_osd_op_vector_in_data: payload_len = the op's input length
            o.encode_raw(&mut b, o.indata.len() as u32);
            data.put_slice(&o.indata);
        }
        b.put_u64_le(self.hobj.snap);
        b.put_u64_le(self.snap_seq);
        self.snaps.encode(&mut b, peer_features);
        b.put_i32_le(self.retry_attempt);
        b.put_u64_le(self.features);
        let mut raw = denc::RawMessage::new(CEPH_MSG_OSD_OP, v, 3, b.freeze());
        raw.tid = self.tid;
        raw.priority = self.priority;
        raw.data = data.freeze();
        Some(raw)
    }

    /// Decode v8/v9 (`MOSDOp::decode_payload` + `finish_decode`), as an OSD would; used by
    /// tests and the mock OSD.
    pub fn decode_raw(raw: &denc::RawMessage) -> Result<Self> {
        if raw.msg_type != CEPH_MSG_OSD_OP {
            return Err(invalid("MOSDOp", format!("type {}", raw.msg_type)));
        }
        let v = raw.version;
        if v != 8 && v != 9 {
            return Err(invalid("MOSDOp", format!("unsupported version {v}")));
        }
        let mut p = raw.front.clone();
        let pgid = SpgId::decode(&mut p)?;
        let hash = u32::decode(&mut p)?;
        let osdmap_epoch = u32::decode(&mut p)?;
        let flags = u32::decode(&mut p)?;
        let reqid = OsdReqId::decode(&mut p)?;
        crate::util::skip(&mut p, 24, "blkin_trace_info")?;
        if v >= 9 {
            let _ = denc::decode_start(&mut p, 254, "jspan_context")?;
        }
        let client_inc = u32::decode(&mut p)?;
        let mtime = UTime::decode(&mut p)?;
        let oloc = ObjectLocator::decode(&mut p)?;
        let oid = Bytes::decode(&mut p)?;
        let num_ops = u16::decode(&mut p)?;
        let mut ops = Vec::with_capacity(num_ops as usize);
        for _ in 0..num_ops {
            ops.push(OsdOp::decode_raw(&mut p)?);
        }
        let snap = u64::decode(&mut p)?;
        let snap_seq = u64::decode(&mut p)?;
        let snaps = Vec::<u64>::decode(&mut p)?;
        let retry_attempt = i32::decode(&mut p)?;
        let features = u64::decode(&mut p)?;
        // split_osd_op_vector_in_data
        let mut data = raw.data.clone();
        for o in ops.iter_mut() {
            if o.payload_len > 0 {
                o.indata = denc::take(&mut data, o.payload_len as usize, "MOSDOp op data")?;
            }
        }
        let hobj = HObject { oid, key: oloc.key.clone(), snap, hash, max: false, nspace: oloc.nspace.clone(), pool: oloc.pool };
        Ok(MOsdOp {
            tid: raw.tid,
            client_inc,
            osdmap_epoch,
            flags,
            mtime,
            retry_attempt,
            hobj,
            pgid,
            ops,
            snap_seq,
            snaps,
            features,
            reqid,
            priority: raw.priority,
        })
    }
}

/// `request_redirect_t`: `ENCODE_START(1, 1)`, `object_locator_t redirect_locator`,
/// `string redirect_object`, `u32 legacy osd_instructions length` (+ bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestRedirect {
    pub locator: ObjectLocator,
    pub object: Bytes,
}

impl Decode for RequestRedirect {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut st = denc::decode_start(buf, 1, "request_redirect_t")?;
        let b = &mut st.body;
        let locator = ObjectLocator::decode(b)?;
        let object = Bytes::decode(b)?;
        let legacy = u32::decode(b)? as usize;
        crate::util::skip(b, legacy, "request_redirect_t legacy")?;
        Ok(RequestRedirect { locator, object })
    }
}

impl Encode for RequestRedirect {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        let s = encode_start(buf, 1, 1);
        self.locator.encode(buf, f);
        self.object.encode(buf, f);
        buf.put_u32_le(0);
        encode_finish(buf, s);
    }
}

impl RequestRedirect {
    /// `combine_with_locator(oloc, oid)`: apply the redirect to an op's target.
    pub fn combine_with_locator(&self, oloc: &mut ObjectLocator, oid: &mut Bytes) {
        oloc.pool = self.locator.pool;
        oloc.key = self.locator.key.clone();
        oloc.nspace = self.locator.nspace.clone();
        oloc.hash = self.locator.hash;
        if !self.object.is_empty() {
            *oid = self.object.clone();
        }
    }
}

/// `MOSDOpReply`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MOsdOpReply {
    pub tid: u64,
    pub oid: Bytes,
    pub pgid: PgId,
    pub flags: i64,
    pub result: i32,
    pub bad_replay_version: EVersion,
    pub osdmap_epoch: u32,
    pub ops: Vec<OsdOp>,
    pub retry_attempt: i32,
    pub replay_version: EVersion,
    pub user_version: u64,
    pub redirect: Option<RequestRedirect>,
}

impl MOsdOpReply {
    /// Decode a reply (`MOSDOpReply::decode_payload`, versions 2..8).
    pub fn decode_raw(raw: &denc::RawMessage) -> Result<Self> {
        if raw.msg_type != CEPH_MSG_OSD_OPREPLY {
            return Err(invalid("MOSDOpReply", format!("type {}", raw.msg_type)));
        }
        let v = raw.version;
        if v < 2 {
            return Err(invalid("MOSDOpReply", format!("unsupported version {v}")));
        }
        let mut p = raw.front.clone();
        let oid = Bytes::decode(&mut p)?;
        let pgid = PgId::decode(&mut p)?;
        let flags = i64::decode(&mut p)?;
        let result = i32::decode(&mut p)?;
        let bad_replay_version = EVersion::decode(&mut p)?;
        let osdmap_epoch = u32::decode(&mut p)?;
        let num_ops = u32::decode(&mut p)? as usize;
        let mut ops = Vec::with_capacity(num_ops.min(p.remaining() / crate::ops::CEPH_OSD_OP_SIZE + 1));
        for _ in 0..num_ops {
            ops.push(OsdOp::decode_raw(&mut p)?);
        }
        let retry_attempt = if v >= 3 { i32::decode(&mut p)? } else { -1 };
        if v >= 4 {
            for o in ops.iter_mut() {
                o.rval = i32::decode(&mut p)?;
            }
            // split_osd_op_vector_out_data
            let mut data = raw.data.clone();
            for o in ops.iter_mut() {
                if o.payload_len > 0 {
                    o.outdata = denc::take(&mut data, o.payload_len as usize, "MOSDOpReply op data")?;
                }
            }
        }
        let (replay_version, user_version) = if v >= 5 {
            (EVersion::decode(&mut p)?, u64::decode(&mut p)?)
        } else {
            (bad_replay_version, bad_replay_version.version)
        };
        let mut redirect = None;
        if v == 6 {
            let r = RequestRedirect::decode(&mut p)?;
            let empty = r.locator.pool == -1 && r.object.is_empty();
            if !empty {
                redirect = Some(r);
            }
        }
        if v >= 7 && bool::decode(&mut p)? {
            redirect = Some(RequestRedirect::decode(&mut p)?);
        }
        // v8: blkin trace (ignored)
        Ok(MOsdOpReply {
            tid: raw.tid,
            oid,
            pgid,
            flags,
            result,
            bad_replay_version,
            osdmap_epoch,
            ops,
            retry_attempt,
            replay_version,
            user_version,
            redirect,
        })
    }

    /// Encode v8 as an OSD does (for tests and the mock OSD).
    pub fn to_raw(&self) -> denc::RawMessage {
        let mut b = BytesMut::new();
        self.oid.encode(&mut b, 0);
        self.pgid.encode(&mut b, 0);
        b.put_i64_le(self.flags);
        b.put_i32_le(self.result);
        self.bad_replay_version.encode(&mut b, 0);
        b.put_u32_le(self.osdmap_epoch);
        b.put_u32_le(self.ops.len() as u32);
        let mut data = BytesMut::new();
        for o in &self.ops {
            o.encode_raw(&mut b, o.outdata.len() as u32);
            data.put_slice(&o.outdata);
        }
        b.put_i32_le(self.retry_attempt);
        for o in &self.ops {
            b.put_i32_le(o.rval);
        }
        self.replay_version.encode(&mut b, 0);
        b.put_u64_le(self.user_version);
        match &self.redirect {
            Some(r) => {
                b.put_u8(1);
                r.encode(&mut b, 0);
            }
            None => b.put_u8(0),
        }
        b.put_bytes(0, 24);
        let mut raw = denc::RawMessage::new(CEPH_MSG_OSD_OPREPLY, 8, 2, b.freeze());
        raw.tid = self.tid;
        raw.data = data.freeze();
        raw
    }
}

/// `CEPH_OSD_BACKOFF_OP_*`.
pub mod backoff_op {
    pub const BLOCK: u8 = 1;
    pub const ACK_BLOCK: u8 = 2;
    pub const UNBLOCK: u8 = 3;
}

/// `MOSDBackoff` (v1): `spg_t pgid`, `u32 map_epoch`, `u8 op`, `u64 id`, `hobject_t begin`,
/// `hobject_t end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MOsdBackoff {
    pub pgid: SpgId,
    pub map_epoch: u32,
    pub op: u8,
    pub id: u64,
    pub begin: HObject,
    pub end: HObject,
}

impl Message for MOsdBackoff {
    const TYPE: u16 = CEPH_MSG_OSD_BACKOFF;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 1;
    // ACK_BLOCK must match the MOSDOps' priority (osd_client_op_priority).
    const PRIORITY: u16 = OSD_CLIENT_OP_PRIORITY;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        self.pgid.encode(&mut b, f);
        b.put_u32_le(self.map_epoch);
        b.put_u8(self.op);
        b.put_u64_le(self.id);
        self.begin.encode(&mut b, f);
        self.end.encode(&mut b, f);
        Payload { front: b, ..Default::default() }
    }

    fn decode_payload(_version: u16, front: Bytes, _middle: Bytes, _data: Bytes) -> Result<Self> {
        let mut p = front;
        Ok(MOsdBackoff {
            pgid: SpgId::decode(&mut p)?,
            map_epoch: u32::decode(&mut p)?,
            op: u8::decode(&mut p)?,
            id: u64::decode(&mut p)?,
            begin: HObject::decode(&mut p)?,
            end: HObject::decode(&mut p)?,
        })
    }
}

/// `MPing`: empty payload, header version 1, compat 0 (sent to OSDs with laggy ops so that a
/// dead connection surfaces).
pub fn mping() -> denc::RawMessage {
    denc::RawMessage::new(CEPH_MSG_PING, 1, 0, Bytes::new())
}
