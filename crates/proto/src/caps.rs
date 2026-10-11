//! `MClientCaps` (`CEPH_MSG_CLIENT_CAPS`, `src/messages/MClientCaps.h`): cap grants, revokes,
//! flushes and their acks, truncation notices, and cap migration.
//!
//! Front, by header version:
//!
//! | v | field |
//! |---|---|
//! | 1 | raw `ceph_mds_caps_head` (92 bytes): `u32 op`, `u64 ino, realm, cap_id`, `u32 seq, issue_seq, caps, wanted, dirty, migrate_seq`, `u64 snap_follows`, `u32 snap_trace_len, uid, gid, mode, nlink, xattr_len`, `u64 xattr_version` |
//! | 1 | 84-byte body: for `EXPORT` the 21-byte `ceph_mds_cap_peer` zero-padded, otherwise `u64 size, max_size, truncate_size`, `u32 truncate_seq`, `mtime`, `atime`, `ctime`, legacy layout (28), `u32 time_warp_seq` |
//! | 1 | `snap_trace_len` bytes of snap trace (IMPORT) |
//! | 2 | `bufferlist flockbl` |
//! | 3 | `ceph_mds_cap_peer` (IMPORT only) |
//! | 4 | `u64 inline_version`, `bufferlist inline_data` |
//! | 5 | `u32 osd_epoch_barrier` |
//! | 6 | `u64 oldest_flush_tid` |
//! | 7 | `u32 caller_uid`, `u32 caller_gid` |
//! | 8 | `string pool_ns` |
//! | 9 | `utime btime`, `u64 change_attr` |
//! | 10 | `u32 flags` (`FLAG_SYNC`, `FLAG_PENDING_CAPSNAP`) |
//! | 11 | `i64 nfiles`, `i64 nsubdirs` (-1 = not valid) |
//! | 12 | `vector<u8> fscrypt_auth`, `vector<u8> fscrypt_file` |
//! | 13 | `u64 subvolume_id` |
//! | 14 | `i32 oserrno` |
//!
//! The xattr blob travels in the **middle** segment and its length must equal `xattr_len`
//! (C++ asserts; this decoder refuses). `header.tid` carries the client's flush tid (FLUSH,
//! FLUSHSNAP, UPDATE with dirty bits) and is echoed in the acks.
//!
//! The 7.x kernel sends v12, libcephfs 20.2 v12 and libcephfs `main` v14; every version is
//! decoded. This crate encodes v14 (compat 1): the kernel reads up to v12 and libcephfs 20.2
//! decodes by version, so both ignore `subvolume_id` and `oserrno`.

use std::fmt;

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode, Message, Payload, RawMessage, Result, UTime};

use crate::msg_type;
use crate::types::{CEPH_INLINE_NONE, FileLayout};
use crate::util::{check_raw, get, invalid};

/// `CEPH_CAP_OP_*`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapOp(pub u32);

impl CapOp {
    /// MDS to client: caps granted (or re-issued).
    pub const GRANT: CapOp = CapOp(0);
    /// MDS to client: caps revoked; the client must answer with a cap message.
    pub const REVOKE: CapOp = CapOp(1);
    /// MDS to client: truncation notice.
    pub const TRUNC: CapOp = CapOp(2);
    /// MDS to client: the cap moved to another rank (peer record).
    pub const EXPORT: CapOp = CapOp(3);
    /// MDS to client: the cap arrived from another rank (peer record, snap trace).
    pub const IMPORT: CapOp = CapOp(4);
    /// Client to MDS: wanted/caps update, revoke ack, flush (with dirty bits).
    pub const UPDATE: CapOp = CapOp(5);
    pub const DROP: CapOp = CapOp(6);
    /// Client to MDS: dirty metadata writeback (kernel).
    pub const FLUSH: CapOp = CapOp(7);
    /// MDS to client: the flush with `tid` is durable.
    pub const FLUSH_ACK: CapOp = CapOp(8);
    /// Client to MDS: snapped metadata writeback.
    pub const FLUSHSNAP: CapOp = CapOp(9);
    pub const FLUSHSNAP_ACK: CapOp = CapOp(10);
    pub const RELEASE: CapOp = CapOp(11);
    pub const RENEW: CapOp = CapOp(12);

    /// `ceph_cap_op_name`.
    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "grant",
            1 => "revoke",
            2 => "trunc",
            3 => "export",
            4 => "import",
            5 => "update",
            6 => "drop",
            7 => "flush",
            8 => "flush_ack",
            9 => "flushsnap",
            10 => "flushsnap_ack",
            11 => "release",
            12 => "renew",
            _ => "???",
        }
    }

    /// Ops the MDS sends.
    pub fn is_mds_op(self) -> bool {
        matches!(self.0, 0 | 1 | 2 | 3 | 4 | 8 | 10)
    }
}

impl fmt::Debug for CapOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// `MClientCaps::FLAG_SYNC`: the client waits for this flush (flush the journal now).
pub const CLIENT_CAPS_FLAG_SYNC: u32 = 1 << 0;
/// `MClientCaps::FLAG_NO_CAPSNAP` (unused).
pub const CLIENT_CAPS_FLAG_NO_CAPSNAP: u32 = 1 << 1;
/// `MClientCaps::FLAG_PENDING_CAPSNAP`: the client still has a cap snap to flush.
pub const CLIENT_CAPS_FLAG_PENDING_CAPSNAP: u32 = 1 << 2;

/// `ceph_mds_cap_peer` (21 raw bytes): the other side of a cap migration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CapPeer {
    pub cap_id: u64,
    pub issue_seq: u32,
    pub mseq: u32,
    /// The peer rank; -1 when the cap is just dropped (EXPORT to nobody).
    pub mds: i32,
    /// `CEPH_CAP_FLAG_AUTH`, `CEPH_CAP_FLAG_RELEASE`.
    pub flags: u8,
}

impl CapPeer {
    pub const LEN: usize = 21;
}

impl Encode for CapPeer {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u64_le(self.cap_id);
        buf.put_u32_le(self.issue_seq);
        buf.put_u32_le(self.mseq);
        buf.put_i32_le(self.mds);
        buf.put_u8(self.flags);
    }
}

impl Decode for CapPeer {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        denc::need(buf, Self::LEN, "ceph_mds_cap_peer")?;
        Ok(CapPeer {
            cap_id: buf.get_u64_le(),
            issue_seq: buf.get_u32_le(),
            mseq: buf.get_u32_le(),
            mds: buf.get_i32_le(),
            flags: buf.get_u8(),
        })
    }
}

/// Size of the raw `ceph_mds_caps_head`.
const HEAD_LEN: usize = 92;
/// Size of `ceph_mds_caps_non_export_body` (the EXPORT body is padded to it).
const BODY_LEN: usize = 84;

/// `MClientCaps`. See the module documentation for the wire format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MClientCaps {
    /// `header.tid`: the client's flush tid (echoed in FLUSH_ACK / FLUSHSNAP_ACK).
    pub tid: u64,
    pub op: CapOp,
    pub ino: u64,
    /// The inode's snap realm.
    pub realm: u64,
    pub cap_id: u64,
    pub seq: u32,
    pub issue_seq: u32,
    /// Issued (MDS) / implemented-after-drop (client) caps.
    pub caps: u32,
    pub wanted: u32,
    /// Bits being flushed.
    pub dirty: u32,
    pub migrate_seq: u32,
    pub snap_follows: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub nlink: u32,
    pub xattr_version: u64,
    pub size: u64,
    pub max_size: u64,
    pub truncate_size: u64,
    pub truncate_seq: u32,
    pub mtime: UTime,
    pub atime: UTime,
    pub ctime: UTime,
    /// The legacy layout of the body plus `pool_ns` (v8).
    pub layout: FileLayout,
    pub time_warp_seq: u32,
    /// EXPORT: the target; IMPORT (v3+): the exporter.
    pub peer: CapPeer,
    /// Snap trace (IMPORT), in the session's format.
    pub snapbl: Bytes,
    /// The xattr blob (middle segment), `u32 count` + pairs; empty when not sent.
    pub xattrbl: Bytes,
    pub flockbl: Bytes,
    /// 0 = no inline data news; `CEPH_INLINE_NONE` = not inline.
    pub inline_version: u64,
    pub inline_data: Bytes,
    /// Clients must not use the caps before they have this OSDMap epoch.
    pub osd_epoch_barrier: u32,
    pub oldest_flush_tid: u64,
    /// libcephfs sends -1 (no path-cap re-check on cap flushes); the kernel 0.
    pub caller_uid: u32,
    pub caller_gid: u32,
    pub btime: UTime,
    pub change_attr: u64,
    /// `CLIENT_CAPS_FLAG_*`.
    pub flags: u32,
    /// Directory entry counts; -1 = not valid.
    pub nfiles: i64,
    pub nsubdirs: i64,
    pub fscrypt_auth: Bytes,
    pub fscrypt_file: Bytes,
    pub subvolume_id: u64,
    pub oserrno: i32,
}

impl Default for MClientCaps {
    /// The C++ defaults: everything zero except `nfiles`/`nsubdirs` (-1) and the layout
    /// (none).
    fn default() -> Self {
        MClientCaps {
            tid: 0,
            op: CapOp::GRANT,
            ino: 0,
            realm: 0,
            cap_id: 0,
            seq: 0,
            issue_seq: 0,
            caps: 0,
            wanted: 0,
            dirty: 0,
            migrate_seq: 0,
            snap_follows: 0,
            uid: 0,
            gid: 0,
            mode: 0,
            nlink: 0,
            xattr_version: 0,
            size: 0,
            max_size: 0,
            truncate_size: 0,
            truncate_seq: 0,
            mtime: UTime::default(),
            atime: UTime::default(),
            ctime: UTime::default(),
            layout: FileLayout::default(),
            time_warp_seq: 0,
            peer: CapPeer::default(),
            snapbl: Bytes::new(),
            xattrbl: Bytes::new(),
            flockbl: Bytes::new(),
            inline_version: 0,
            inline_data: Bytes::new(),
            osd_epoch_barrier: 0,
            oldest_flush_tid: 0,
            caller_uid: 0,
            caller_gid: 0,
            btime: UTime::default(),
            change_attr: 0,
            flags: 0,
            nfiles: -1,
            nsubdirs: -1,
            fscrypt_auth: Bytes::new(),
            fscrypt_file: Bytes::new(),
            subvolume_id: 0,
            oserrno: 0,
        }
    }
}

/// The inode fields a GRANT/REVOKE/TRUNC/IMPORT carries (`CInode::encode_cap_message`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapInodeFields {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub nlink: u32,
    pub size: u64,
    pub max_size: u64,
    pub truncate_size: u64,
    pub truncate_seq: u32,
    pub mtime: UTime,
    pub atime: UTime,
    pub ctime: UTime,
    pub btime: UTime,
    pub change_attr: u64,
    pub time_warp_seq: u32,
    pub layout: FileLayout,
    pub nfiles: i64,
    pub nsubdirs: i64,
    /// `CEPH_INLINE_NONE` the first time, 0 later ("no news"), as C++ tracks per cap.
    pub inline_version: u64,
    /// The xattr blob and its version, when the client's copy is older and it holds `Xs`.
    pub xattrs: Option<(u64, Bytes)>,
    pub fscrypt_auth: Bytes,
    pub fscrypt_file: Bytes,
}

impl MClientCaps {
    /// `MClientCaps(op, ino, realm, id, seq, caps, wanted, dirty, mseq, issue_seq, oeb)`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        op: CapOp,
        ino: u64,
        realm: u64,
        cap_id: u64,
        seq: u32,
        caps: u32,
        wanted: u32,
        dirty: u32,
        migrate_seq: u32,
        issue_seq: u32,
        osd_epoch_barrier: u32,
    ) -> Self {
        MClientCaps {
            op,
            ino,
            realm,
            cap_id,
            seq,
            caps,
            wanted,
            dirty,
            migrate_seq,
            issue_seq,
            osd_epoch_barrier,
            ..Default::default()
        }
    }

    /// Copy the inode fields of a GRANT/REVOKE/TRUNC/IMPORT.
    pub fn set_inode(&mut self, f: &CapInodeFields) {
        self.uid = f.uid;
        self.gid = f.gid;
        self.mode = f.mode;
        self.nlink = f.nlink;
        self.size = f.size;
        self.max_size = f.max_size;
        self.truncate_size = f.truncate_size;
        self.truncate_seq = f.truncate_seq;
        self.mtime = f.mtime;
        self.atime = f.atime;
        self.ctime = f.ctime;
        self.btime = f.btime;
        self.change_attr = f.change_attr;
        self.time_warp_seq = f.time_warp_seq;
        self.layout = f.layout.clone();
        self.nfiles = f.nfiles;
        self.nsubdirs = f.nsubdirs;
        self.inline_version = f.inline_version;
        if let Some((v, blob)) = &f.xattrs {
            self.xattr_version = *v;
            self.xattrbl = blob.clone();
        }
        self.fscrypt_auth = f.fscrypt_auth.clone();
        self.fscrypt_file = f.fscrypt_file.clone();
    }

    /// `FLUSH_ACK` for a client flush (`Locker::handle_client_caps`): echoes the flush `tid`,
    /// `cap_id`, `seq`, `caps` and the flushed `dirty` bits.
    #[allow(clippy::too_many_arguments)]
    pub fn flush_ack(
        ino: u64,
        cap_id: u64,
        seq: u32,
        caps: u32,
        dirty: u32,
        issue_seq: u32,
        client_tid: u64,
        oldest_flush_tid: u64,
        osd_epoch_barrier: u32,
    ) -> Self {
        MClientCaps {
            tid: client_tid,
            oldest_flush_tid,
            ..Self::new(CapOp::FLUSH_ACK, ino, 0, cap_id, seq, caps, 0, dirty, 0, issue_seq, osd_epoch_barrier)
        }
    }

    /// `FLUSHSNAP_ACK`: echoes `snap_follows` and the flush `tid`. Send it only from a rank
    /// where the client holds a cap on the inode (libcephfs asserts).
    pub fn flushsnap_ack(
        ino: u64,
        dirty: u32,
        follows: u64,
        client_tid: u64,
        oldest_flush_tid: u64,
        osd_epoch_barrier: u32,
    ) -> Self {
        MClientCaps {
            tid: client_tid,
            snap_follows: follows,
            oldest_flush_tid,
            ..Self::new(CapOp::FLUSHSNAP_ACK, ino, 0, 0, 0, 0, 0, dirty, 0, 0, osd_epoch_barrier)
        }
    }

    /// `EXPORT`: the cap moved to `peer.mds` (`peer.mds = -1`, `cap_id 0`: just drop it).
    pub fn export(ino: u64, cap_id: u64, migrate_seq: u32, peer: CapPeer, osd_epoch_barrier: u32) -> Self {
        MClientCaps { peer, ..Self::new(CapOp::EXPORT, ino, 0, cap_id, 0, 0, 0, 0, migrate_seq, 0, osd_epoch_barrier) }
    }

    /// The header version [`MClientCaps::to_raw`] writes.
    pub const fn encode_version() -> u16 {
        Self::HEAD_VERSION
    }

    /// Encode at header version `version` (2..=14; later fields are left out). Use
    /// [`Message::encode_payload`] (v14) except to reproduce older senders' bytes.
    pub fn encode_payload_version(&self, version: u16) -> Payload {
        let version = version.clamp(2, Self::HEAD_VERSION);
        let mut b = BytesMut::with_capacity(HEAD_LEN + BODY_LEN + 128 + self.snapbl.len());
        b.put_u32_le(self.op.0);
        b.put_u64_le(self.ino);
        b.put_u64_le(self.realm);
        b.put_u64_le(self.cap_id);
        b.put_u32_le(self.seq);
        b.put_u32_le(self.issue_seq);
        b.put_u32_le(self.caps);
        b.put_u32_le(self.wanted);
        b.put_u32_le(self.dirty);
        b.put_u32_le(self.migrate_seq);
        b.put_u64_le(self.snap_follows);
        b.put_u32_le(self.snapbl.len() as u32);
        b.put_u32_le(self.uid);
        b.put_u32_le(self.gid);
        b.put_u32_le(self.mode);
        b.put_u32_le(self.nlink);
        b.put_u32_le(self.xattrbl.len() as u32);
        b.put_u64_le(self.xattr_version);
        if self.op == CapOp::EXPORT {
            self.peer.encode(&mut b, 0);
            b.put_bytes(0, BODY_LEN - CapPeer::LEN);
        } else {
            b.put_u64_le(self.size);
            b.put_u64_le(self.max_size);
            b.put_u64_le(self.truncate_size);
            b.put_u32_le(self.truncate_seq);
            self.mtime.encode(&mut b, 0);
            self.atime.encode(&mut b, 0);
            self.ctime.encode(&mut b, 0);
            self.layout.encode_legacy(&mut b);
            b.put_u32_le(self.time_warp_seq);
        }
        b.put_slice(&self.snapbl);
        self.flockbl.encode(&mut b, 0);
        if version >= 3 && self.op == CapOp::IMPORT {
            self.peer.encode(&mut b, 0);
        }
        if version >= 4 {
            b.put_u64_le(self.inline_version);
            self.inline_data.encode(&mut b, 0);
        }
        if version >= 5 {
            b.put_u32_le(self.osd_epoch_barrier);
        }
        if version >= 6 {
            b.put_u64_le(self.oldest_flush_tid);
        }
        if version >= 7 {
            b.put_u32_le(self.caller_uid);
            b.put_u32_le(self.caller_gid);
        }
        if version >= 8 {
            self.layout.pool_ns.encode(&mut b, 0);
        }
        if version >= 9 {
            self.btime.encode(&mut b, 0);
            b.put_u64_le(self.change_attr);
        }
        if version >= 10 {
            b.put_u32_le(self.flags);
        }
        if version >= 11 {
            b.put_i64_le(self.nfiles);
            b.put_i64_le(self.nsubdirs);
        }
        if version >= 12 {
            self.fscrypt_auth.encode(&mut b, 0);
            self.fscrypt_file.encode(&mut b, 0);
        }
        if version >= 13 {
            b.put_u64_le(self.subvolume_id);
        }
        if version >= 14 {
            b.put_i32_le(self.oserrno);
        }
        Payload { front: b, middle: self.xattrbl.clone(), data: Bytes::new() }
    }
}

impl Message for MClientCaps {
    const TYPE: u16 = msg_type::CLIENT_CAPS;
    const HEAD_VERSION: u16 = 14;
    const COMPAT_VERSION: u16 = 1;

    fn encode_payload(&self, _features: u64) -> Payload {
        self.encode_payload_version(Self::HEAD_VERSION)
    }

    fn decode_payload(version: u16, mut front: Bytes, middle: Bytes, _data: Bytes) -> Result<Self> {
        let p = &mut front;
        denc::need(p, HEAD_LEN + BODY_LEN, "ceph_mds_caps")?;
        let mut m = MClientCaps {
            op: CapOp(p.get_u32_le()),
            ino: p.get_u64_le(),
            realm: p.get_u64_le(),
            cap_id: p.get_u64_le(),
            seq: p.get_u32_le(),
            issue_seq: p.get_u32_le(),
            caps: p.get_u32_le(),
            wanted: p.get_u32_le(),
            dirty: p.get_u32_le(),
            migrate_seq: p.get_u32_le(),
            snap_follows: p.get_u64_le(),
            ..Default::default()
        };
        let snap_trace_len = p.get_u32_le() as usize;
        m.uid = p.get_u32_le();
        m.gid = p.get_u32_le();
        m.mode = p.get_u32_le();
        m.nlink = p.get_u32_le();
        let xattr_len = p.get_u32_le() as usize;
        m.xattr_version = p.get_u64_le();
        if m.op == CapOp::EXPORT {
            m.peer = CapPeer::decode(p)?;
            p.advance(BODY_LEN - CapPeer::LEN);
        } else {
            m.size = p.get_u64_le();
            m.max_size = p.get_u64_le();
            m.truncate_size = p.get_u64_le();
            m.truncate_seq = p.get_u32_le();
            m.mtime = UTime::decode(p)?;
            m.atime = UTime::decode(p)?;
            m.ctime = UTime::decode(p)?;
            m.layout = FileLayout::decode_legacy(p)?;
            m.time_warp_seq = p.get_u32_le();
        }
        m.snapbl = denc::take(p, snap_trace_len, "MClientCaps.snap_trace")?;
        if middle.len() != xattr_len {
            return Err(invalid(
                "MClientCaps",
                format!("xattr_len {xattr_len} differs from the middle segment length {}", middle.len()),
            ));
        }
        m.xattrbl = middle;
        if version >= 2 {
            m.flockbl = get(p, "MClientCaps.flockbl")?;
        }
        if version >= 3 && m.op == CapOp::IMPORT {
            m.peer = CapPeer::decode(p)?;
        }
        if version >= 4 {
            m.inline_version = get(p, "MClientCaps.inline_version")?;
            m.inline_data = get(p, "MClientCaps.inline_data")?;
        } else {
            m.inline_version = CEPH_INLINE_NONE;
        }
        if version >= 5 {
            m.osd_epoch_barrier = get(p, "MClientCaps.osd_epoch_barrier")?;
        }
        if version >= 6 {
            m.oldest_flush_tid = get(p, "MClientCaps.oldest_flush_tid")?;
        }
        if version >= 7 {
            m.caller_uid = get(p, "MClientCaps.caller_uid")?;
            m.caller_gid = get(p, "MClientCaps.caller_gid")?;
        }
        if version >= 8 {
            m.layout.pool_ns = String::from_utf8_lossy(&get::<Bytes>(p, "MClientCaps.pool_ns")?).into_owned();
        }
        if version >= 9 {
            m.btime = get(p, "MClientCaps.btime")?;
            m.change_attr = get(p, "MClientCaps.change_attr")?;
        }
        if version >= 10 {
            m.flags = get(p, "MClientCaps.flags")?;
        }
        if version >= 11 {
            m.nfiles = get(p, "MClientCaps.nfiles")?;
            m.nsubdirs = get(p, "MClientCaps.nsubdirs")?;
        }
        if version >= 12 {
            m.fscrypt_auth = get(p, "MClientCaps.fscrypt_auth")?;
            m.fscrypt_file = get(p, "MClientCaps.fscrypt_file")?;
        }
        if version >= 13 {
            m.subvolume_id = get(p, "MClientCaps.subvolume_id")?;
        }
        if version >= 14 {
            m.oserrno = get(p, "MClientCaps.oserrno")?;
        }
        Ok(m)
    }

    fn tid(&self) -> u64 {
        self.tid
    }

    fn from_raw(raw: RawMessage) -> Result<Self> {
        check_raw::<Self>(&raw)?;
        let tid = raw.tid;
        let mut m = Self::decode_payload(raw.version, raw.front, raw.middle, raw.data)?;
        m.tid = tid;
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snap::{SnapFormat, SnapRealmInfo, SnapTrace};
    use crate::types::{CEPH_CAP_FLAG_AUTH, XattrMap, cap, xattrs_to_blob};

    fn grant() -> MClientCaps {
        let mut x = XattrMap::new();
        x.insert(Bytes::from_static(b"user.a"), Bytes::from_static(b"1"));
        let mut m = MClientCaps::new(
            CapOp::GRANT,
            0x100_0000_0001,
            1,
            5,
            3,
            cap::PIN | cap::ANY_SHARED | cap::FILE_RD | cap::FILE_CACHE,
            cap::ANY_FILE_RD,
            0,
            0,
            2,
            17,
        );
        m.set_inode(&CapInodeFields {
            uid: 1000,
            gid: 1000,
            mode: 0o100644,
            nlink: 1,
            size: 4096,
            max_size: 0,
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
        m
    }

    #[test]
    fn layout_offsets() {
        let m = grant();
        let raw = m.to_raw(0);
        assert_eq!((raw.version, raw.compat_version), (14, 1));
        let f = &raw.front;
        // Head and body add up to the kernel's 176-byte struct ceph_mds_caps.
        assert_eq!(&f[0..4], &0u32.to_le_bytes());
        assert_eq!(&f[4..12], &m.ino.to_le_bytes());
        assert_eq!(&f[60..64], &0u32.to_le_bytes()); // snap_trace_len
        assert_eq!(&f[80..84], &(m.xattrbl.len() as u32).to_le_bytes());
        assert_eq!(&f[84..92], &2u64.to_le_bytes()); // xattr_version
        assert_eq!(&f[92..100], &4096u64.to_le_bytes()); // size
        assert_eq!(&f[172..176], &1u32.to_le_bytes()); // time_warp_seq
        assert_eq!(&f[176..180], &0u32.to_le_bytes()); // empty flock blob
        assert_eq!(&f[180..188], &u64::MAX.to_le_bytes()); // inline_version
        assert_eq!(&f[192..196], &17u32.to_le_bytes()); // osd_epoch_barrier
        assert_eq!(raw.middle, m.xattrbl);
        // Tail: ..., nfiles/nsubdirs -1, fscrypt, subvolume_id, oserrno.
        let n = f.len();
        assert_eq!(&f[n - 4..], &0i32.to_le_bytes());
        assert_eq!(&f[n - 12..n - 4], &0u64.to_le_bytes());
        assert_eq!(&f[n - 20..n - 12], &[0u8; 8]); // two empty vectors
        assert_eq!(&f[n - 28..n - 20], &(-1i64).to_le_bytes());
        let back = MClientCaps::from_raw(raw).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn v12_is_a_prefix_of_v14() {
        let m = grant();
        let v12 = m.encode_payload_version(12);
        let v14 = m.encode_payload(0);
        assert_eq!(&v14.front[..v12.front.len()], &v12.front[..]);
        assert_eq!(v14.front.len(), v12.front.len() + 12);
        let back = MClientCaps::decode_payload(12, v12.front.freeze(), v12.middle, Bytes::new()).unwrap();
        assert_eq!(back.subvolume_id, 0);
        assert_eq!(back.caps, m.caps);
        assert_eq!(back.layout.pool_ns, "ns");
    }

    #[test]
    fn export_and_import() {
        let peer = CapPeer { cap_id: 9, issue_seq: 2, mseq: 3, mds: 1, flags: CEPH_CAP_FLAG_AUTH };
        let m = MClientCaps::export(0x1000, 5, 2, peer, 0);
        let raw = m.to_raw(0);
        assert_eq!(&raw.front[92..100], &9u64.to_le_bytes());
        assert_eq!(&raw.front[112..113], &[CEPH_CAP_FLAG_AUTH]);
        assert!(raw.front[113..176].iter().all(|b| *b == 0));
        let back = MClientCaps::from_raw(raw).unwrap();
        assert_eq!(back.peer, peer);
        assert_eq!(back.size, 0);
        let mut imp = grant();
        imp.op = CapOp::IMPORT;
        imp.peer = peer;
        imp.snapbl = SnapTrace::new(vec![SnapRealmInfo::new(1, 0, 1)]).encode(SnapFormat::Old);
        let raw = imp.to_raw(0);
        assert_eq!(&raw.front[60..64], &48u32.to_le_bytes());
        // flock blob, then the import peer record
        let off = 176 + 48 + 4;
        assert_eq!(&raw.front[off..off + 8], &9u64.to_le_bytes());
        assert_eq!(MClientCaps::from_raw(raw).unwrap(), imp);
    }

    #[test]
    fn acks() {
        let a = MClientCaps::flush_ack(0x1000, 5, 7, cap::PIN, cap::FILE_WR, 3, 42, 40, 0);
        let raw = a.to_raw(0);
        assert_eq!(raw.tid, 42);
        let back = MClientCaps::from_raw(raw).unwrap();
        assert_eq!(
            (back.op, back.tid, back.dirty, back.seq, back.oldest_flush_tid),
            (CapOp::FLUSH_ACK, 42, cap::FILE_WR, 7, 40)
        );
        assert_eq!((back.nfiles, back.nsubdirs), (-1, -1));
        let s = MClientCaps::flushsnap_ack(0x1000, cap::FILE_WR, 3, 43, 40, 0);
        let back = MClientCaps::from_raw(s.to_raw(0)).unwrap();
        assert_eq!((back.op, back.snap_follows, back.tid), (CapOp::FLUSHSNAP_ACK, 3, 43));
        assert!(CapOp::FLUSH_ACK.is_mds_op() && !CapOp::UPDATE.is_mds_op());
    }

    /// The kernel's v12 UPDATE layout (kclient 3.9.3): flock 0, inline_version, inline len,
    /// barrier, oldest flush tid, caller ids 0, empty pool_ns, btime, change_attr, flags,
    /// dirstats 0/0, fscrypt auth, fscrypt file (len 8 + real size).
    #[test]
    fn decode_kernel_v12_update() {
        let mut m = MClientCaps::new(
            CapOp::UPDATE,
            0x100_0000_0005,
            1,
            77,
            4,
            cap::PIN | cap::FILE_SHARED,
            cap::FILE_RD,
            cap::FILE_WR,
            0,
            4,
            0,
        );
        m.tid = 1001;
        m.size = 12345;
        m.flags = CLIENT_CAPS_FLAG_SYNC;
        m.nfiles = 0;
        m.nsubdirs = 0;
        m.inline_version = CEPH_INLINE_NONE;
        m.oldest_flush_tid = 1000;
        m.fscrypt_file = Bytes::from_static(&[0x39, 0x30, 0, 0, 0, 0, 0, 0]);
        let p = m.encode_payload_version(12);
        let mut raw = RawMessage::new(msg_type::CLIENT_CAPS, 12, 1, p.front.freeze());
        raw.tid = 1001;
        let back = MClientCaps::from_raw(raw).unwrap();
        assert_eq!(back, m);
        // xattr_len must match the middle segment.
        let p = m.encode_payload_version(12);
        let mut raw = RawMessage::new(msg_type::CLIENT_CAPS, 12, 1, p.front.freeze());
        raw.middle = Bytes::from_static(&[0, 0, 0, 0]);
        assert!(MClientCaps::from_raw(raw).is_err());
        // Truncated fronts fail cleanly at every length.
        let full = m.encode_payload_version(12).front.freeze();
        for cut in 0..full.len() {
            assert!(
                MClientCaps::decode_payload(12, full.slice(..cut), Bytes::new(), Bytes::new()).is_err(),
                "cut {cut}"
            );
        }
    }
}
