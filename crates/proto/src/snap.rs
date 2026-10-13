//! Snap realm traces and `MClientSnap` (`CEPH_MSG_CLIENT_SNAP`).
//!
//! A snap trace describes a realm and its ancestors up to the root realm, as a plain
//! concatenation of realm records with no count (`SnapRealm::build_snap_trace`,
//! `src/mds/SnapRealm.cc`). Two record formats exist and the MDS picks one per session
//! (`Server::get_snap_trace`): the old `SnapRealmInfo` for sessions without
//! `CEPHFS_FEATURE_NEW_SNAPREALM_INFO` (bit 19; every kernel) and `SnapRealmInfoNew` for
//! sessions with it (libcephfs). A malformed trace makes the kernel blocklist itself.

use std::fmt;

use denc::{Buf, BufMut, Bytes, BytesMut, Encode, Message, Payload, Result, UTime};

use crate::features::{FeatureBitset, bit};
use crate::msg_type;
use crate::util::{Ctx, get, get_u64s_nohead, impl_from_raw, invalid};

/// Which realm record format a session gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SnapFormat {
    /// `SnapRealmInfo`: raw `ceph_mds_snap_realm` + snap ids (kernel clients).
    Old,
    /// `SnapRealmInfoNew`: `ENCODE_START(2, 1)` around the old record plus `last_modified`,
    /// `change_attr` and `flags` (libcephfs, which sets bit 19); what Ceph `main` sends.
    New,
    /// `SnapRealmInfoNew` v1, without `flags`: what Ceph 20.2 sends. Decodes like
    /// [`SnapFormat::New`]; both libcephfs 20.2 and `main` accept either version.
    NewV1,
}

impl SnapFormat {
    /// The format for a session with these client CephFS features.
    pub fn for_client(client_features: &FeatureBitset) -> SnapFormat {
        if client_features.test(bit::NEW_SNAPREALM_INFO) {
            SnapFormat::New
        } else {
            SnapFormat::Old
        }
    }
}

/// `SnapRealmInfoNew::SNAPDIR_VISIBILITY`.
pub const SNAPREALM_SNAPDIR_VISIBILITY: u32 = 4;

/// One realm of a snap trace (`SnapRealmInfo` / `SnapRealmInfoNew`, `src/common/snap_types.cc`).
///
/// Old wire form: raw `ceph_mds_snap_realm` (`u64 ino, created, parent, parent_since, seq`,
/// `u32 num_snaps, num_prior_parent_snaps`; 48 bytes), then `num_snaps` and
/// `num_prior_parent_snaps` `u64` snap ids. New wire form: `ENCODE_START(2, 1)` { old form,
/// `utime last_modified`, `u64 change_attr`, `u32 flags` (v2) }.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct SnapRealmInfo {
    /// Realm root inode.
    pub ino: u64,
    /// Snapid when the realm was created.
    pub created: u64,
    /// Parent realm inode (0 for the root realm).
    pub parent: u64,
    /// Snapid since which `parent` is the parent.
    pub parent_since: u64,
    /// Realm version (newest snapid relevant to it).
    pub seq: u64,
    /// The realm's own snapshots, descending.
    pub my_snaps: Vec<u64>,
    /// Snapshots inherited from former parents, descending.
    pub prior_parent_snaps: Vec<u64>,
    /// New format only.
    pub last_modified: UTime,
    /// New format only.
    pub change_attr: u64,
    /// New format only ([`SNAPREALM_SNAPDIR_VISIBILITY`]).
    pub flags: u32,
}

impl fmt::Debug for SnapRealmInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnapRealmInfo")
            .field("ino", &format_args!("{:#x}", self.ino))
            .field("created", &self.created)
            .field("parent", &format_args!("{:#x}", self.parent))
            .field("parent_since", &self.parent_since)
            .field("seq", &self.seq)
            .field("my_snaps", &self.my_snaps)
            .field("prior_parent_snaps", &self.prior_parent_snaps)
            .field("last_modified", &self.last_modified)
            .field("change_attr", &self.change_attr)
            .field("flags", &self.flags)
            .finish()
    }
}

impl SnapRealmInfo {
    /// A realm with no snapshots (what P0 sends: realms exist for the root, quota directories
    /// and subvolume roots, but no snapshots are taken).
    pub fn new(ino: u64, parent: u64, seq: u64) -> Self {
        SnapRealmInfo {
            ino,
            parent,
            seq,
            ..Default::default()
        }
    }

    fn encode_old(&self, buf: &mut BytesMut) {
        buf.put_u64_le(self.ino);
        buf.put_u64_le(self.created);
        buf.put_u64_le(self.parent);
        buf.put_u64_le(self.parent_since);
        buf.put_u64_le(self.seq);
        buf.put_u32_le(self.my_snaps.len() as u32);
        buf.put_u32_le(self.prior_parent_snaps.len() as u32);
        for s in &self.my_snaps {
            buf.put_u64_le(*s);
        }
        for s in &self.prior_parent_snaps {
            buf.put_u64_le(*s);
        }
    }

    fn decode_old(buf: &mut Bytes) -> Result<Self> {
        denc::need(buf, 48, "ceph_mds_snap_realm")?;
        let ino = buf.get_u64_le();
        let created = buf.get_u64_le();
        let parent = buf.get_u64_le();
        let parent_since = buf.get_u64_le();
        let seq = buf.get_u64_le();
        let num_snaps = buf.get_u32_le() as usize;
        let num_prior = buf.get_u32_le() as usize;
        let my_snaps = get_u64s_nohead(buf, num_snaps, "SnapRealmInfo.my_snaps")?;
        let prior_parent_snaps =
            get_u64s_nohead(buf, num_prior, "SnapRealmInfo.prior_parent_snaps")?;
        Ok(SnapRealmInfo {
            ino,
            created,
            parent,
            parent_since,
            seq,
            my_snaps,
            prior_parent_snaps,
            ..Default::default()
        })
    }

    /// Append this record in the given format.
    pub fn encode_format(&self, buf: &mut BytesMut, format: SnapFormat) {
        match format {
            SnapFormat::Old => self.encode_old(buf),
            SnapFormat::New => denc::encode_versioned(buf, 2, 1, |b| {
                self.encode_old(b);
                self.last_modified.encode(b, 0);
                b.put_u64_le(self.change_attr);
                b.put_u32_le(self.flags);
            }),
            SnapFormat::NewV1 => denc::encode_versioned(buf, 1, 1, |b| {
                self.encode_old(b);
                self.last_modified.encode(b, 0);
                b.put_u64_le(self.change_attr);
            }),
        }
    }

    /// Decode one record in the given format.
    pub fn decode_format(buf: &mut Bytes, format: SnapFormat) -> Result<Self> {
        match format {
            SnapFormat::Old => Self::decode_old(buf),
            SnapFormat::New | SnapFormat::NewV1 => {
                let mut f = denc::decode_start(buf, 2, "SnapRealmInfoNew")?;
                let b = &mut f.body;
                let mut info = Self::decode_old(b)?;
                info.last_modified = get(b, "SnapRealmInfoNew.last_modified")?;
                info.change_attr = get(b, "SnapRealmInfoNew.change_attr")?;
                if f.v >= 2 {
                    info.flags = get(b, "SnapRealmInfoNew.flags")?;
                }
                Ok(info)
            }
        }
    }
}

/// A snap trace: the realm of interest first, then each parent up to the root realm.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SnapTrace {
    pub realms: Vec<SnapRealmInfo>,
}

impl SnapTrace {
    pub fn new(realms: Vec<SnapRealmInfo>) -> Self {
        SnapTrace { realms }
    }

    /// Encode as the blob carried in replies, `MClientSnap` and cap `IMPORT`s.
    pub fn encode(&self, format: SnapFormat) -> Bytes {
        let mut b = BytesMut::new();
        for r in &self.realms {
            r.encode_format(&mut b, format);
        }
        b.freeze()
    }

    /// Decode a whole blob (records until the end).
    pub fn decode(mut blob: Bytes, format: SnapFormat) -> Result<Self> {
        let mut realms = Vec::new();
        while blob.has_remaining() {
            realms.push(SnapRealmInfo::decode_format(&mut blob, format).ctx("snap trace")?);
        }
        Ok(SnapTrace { realms })
    }

    /// Check the shape a client relies on: each record's parent is the next record's realm,
    /// and the last record is a root (parent 0).
    pub fn is_well_formed(&self) -> bool {
        let Some(last) = self.realms.last() else {
            return true;
        };
        last.parent == 0 && self.realms.windows(2).all(|w| w[0].parent == w[1].ino)
    }
}

/// `CEPH_SNAP_OP_*`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnapOp(pub u32);

impl SnapOp {
    pub const UPDATE: SnapOp = SnapOp(0);
    pub const CREATE: SnapOp = SnapOp(1);
    pub const DESTROY: SnapOp = SnapOp(2);
    pub const SPLIT: SnapOp = SnapOp(3);

    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "update",
            1 => "create",
            2 => "destroy",
            3 => "split",
            _ => "???",
        }
    }
}

impl fmt::Debug for SnapOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// `MClientSnap` (`src/messages/MClientSnap.h`), version 1, MDS to client, counted in
/// `push_seq`.
///
/// Front: raw `ceph_mds_snap_head` (`u32 op`, `u64 split`, `u32 num_split_inos`, `u32
/// num_split_realms`, `u32 trace_len`; 24 bytes), the split inode numbers, the split realm
/// numbers, then the snap trace. Nothing may follow the trace (the kernel parses the trace to
/// the end of the front; libcephfs asserts the end).
///
/// The C++ MDS sends per-realm notifications as `SPLIT` (with empty lists unless a realm is
/// really being split) and global-realm notifications as `UPDATE`/`DESTROY`; it never sends
/// `CREATE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MClientSnap {
    pub op: SnapOp,
    /// For `SPLIT`: the realm inode the listed inodes and child realms move to.
    pub split: u64,
    pub split_inos: Vec<u64>,
    pub split_realms: Vec<u64>,
    /// The snap trace, starting at the realm `split` names, in the session's format.
    pub trace: Bytes,
}

impl MClientSnap {
    pub fn new(op: SnapOp, split: u64, trace: Bytes) -> Self {
        MClientSnap {
            op,
            split,
            split_inos: Vec::new(),
            split_realms: Vec::new(),
            trace,
        }
    }

    /// A realm split: `inos` and child `realms` move under the new realm `realm`; `trace`
    /// starts at `realm`.
    pub fn split(realm: u64, inos: Vec<u64>, realms: Vec<u64>, trace: Bytes) -> Self {
        MClientSnap {
            op: SnapOp::SPLIT,
            split: realm,
            split_inos: inos,
            split_realms: realms,
            trace,
        }
    }

    /// A realm update notification.
    pub fn update(realm: u64, trace: Bytes) -> Self {
        MClientSnap::new(SnapOp::UPDATE, realm, trace)
    }
}

impl Message for MClientSnap {
    const TYPE: u16 = msg_type::CLIENT_SNAP;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, _features: u64) -> Payload {
        let mut b = BytesMut::with_capacity(
            24 + 8 * (self.split_inos.len() + self.split_realms.len()) + self.trace.len(),
        );
        b.put_u32_le(self.op.0);
        b.put_u64_le(self.split);
        b.put_u32_le(self.split_inos.len() as u32);
        b.put_u32_le(self.split_realms.len() as u32);
        b.put_u32_le(self.trace.len() as u32);
        for i in &self.split_inos {
            b.put_u64_le(*i);
        }
        for r in &self.split_realms {
            b.put_u64_le(*r);
        }
        b.put_slice(&self.trace);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(
        _version: u16,
        mut front: Bytes,
        _middle: Bytes,
        _data: Bytes,
    ) -> Result<Self> {
        let p = &mut front;
        denc::need(p, 24, "ceph_mds_snap_head")?;
        let op = SnapOp(p.get_u32_le());
        let split = p.get_u64_le();
        let num_inos = p.get_u32_le() as usize;
        let num_realms = p.get_u32_le() as usize;
        let trace_len = p.get_u32_le() as usize;
        let split_inos = get_u64s_nohead(p, num_inos, "MClientSnap.split_inos")?;
        let split_realms = get_u64s_nohead(p, num_realms, "MClientSnap.split_realms")?;
        let trace = denc::take(p, trace_len, "MClientSnap.trace")?;
        if p.has_remaining() {
            return Err(invalid(
                "MClientSnap",
                format!("{} trailing bytes", p.remaining()),
            ));
        }
        Ok(MClientSnap {
            op,
            split,
            split_inos,
            split_realms,
            trace,
        })
    }

    impl_from_raw!();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SnapTrace {
        SnapTrace::new(vec![
            SnapRealmInfo {
                ino: 0x1000,
                created: 2,
                parent: 1,
                parent_since: 2,
                seq: 5,
                my_snaps: vec![5, 3],
                prior_parent_snaps: vec![1],
                last_modified: UTime { sec: 10, nsec: 20 },
                change_attr: 7,
                flags: SNAPREALM_SNAPDIR_VISIBILITY,
            },
            SnapRealmInfo::new(1, 0, 1),
        ])
    }

    #[test]
    fn old_format_bytes() {
        let t = SnapTrace::new(vec![SnapRealmInfo::new(1, 0, 1)]);
        let b = t.encode(SnapFormat::Old);
        assert_eq!(
            hex::encode(&b),
            "0100000000000000".to_owned()
                + "0000000000000000"
                + "0000000000000000"
                + "0000000000000000"
                + "0100000000000000"
                + "00000000"
                + "00000000"
        );
        let full = sample();
        let b = full.encode(SnapFormat::Old);
        assert_eq!(b.len(), 48 + 3 * 8 + 48);
        let back = SnapTrace::decode(b, SnapFormat::Old).unwrap();
        // The old format has no last_modified/change_attr/flags.
        assert_eq!(back.realms[0].my_snaps, vec![5, 3]);
        assert_eq!(back.realms[0].prior_parent_snaps, vec![1]);
        assert_eq!(back.realms[0].change_attr, 0);
        assert_eq!(back.realms[1], full.realms[1]);
        assert!(back.is_well_formed());
    }

    #[test]
    fn new_format_round_trip() {
        let t = sample();
        let b = t.encode(SnapFormat::New);
        // ENCODE_START(2,1) header of the first record.
        assert_eq!(&b[..2], &[2, 1]);
        let first_len = u32::from_le_bytes(b[2..6].try_into().unwrap()) as usize;
        assert_eq!(first_len, 48 + 24 + 8 + 8 + 4);
        assert_eq!(SnapTrace::decode(b, SnapFormat::New).unwrap(), t);
        // The Ceph 20.2 form: struct v1 without flags (8 + 8 + 48 + 24 = 64 + 24 bytes body).
        let b = t.encode(SnapFormat::NewV1);
        assert_eq!(&b[..6], &[1, 1, 88, 0, 0, 0]);
        let back = SnapTrace::decode(b, SnapFormat::NewV1).unwrap();
        assert_eq!(back.realms[0].flags, 0);
        assert_eq!(back.realms[0].change_attr, 7);
    }

    #[test]
    fn malformed_traces() {
        let t = sample();
        let mut b = BytesMut::from(&t.encode(SnapFormat::Old)[..]);
        b.truncate(b.len() - 1);
        assert!(SnapTrace::decode(b.freeze(), SnapFormat::Old).is_err());
        let bad = SnapTrace::new(vec![
            SnapRealmInfo::new(0x1000, 7, 1),
            SnapRealmInfo::new(1, 0, 1),
        ]);
        assert!(!bad.is_well_formed());
        assert!(SnapTrace::default().is_well_formed());
    }

    #[test]
    fn mclientsnap_round_trip() {
        let trace = sample().encode(SnapFormat::Old);
        let m = MClientSnap::split(0x1000, vec![0x1001, 0x1002], vec![0x2000], trace.clone());
        let raw = m.to_raw(0);
        assert_eq!((raw.version, raw.compat_version), (1, 0));
        assert_eq!(&raw.front[..4], &3u32.to_le_bytes());
        assert_eq!(&raw.front[20..24], &(trace.len() as u32).to_le_bytes());
        assert_eq!(raw.front.len(), 24 + 24 + trace.len());
        let back = MClientSnap::from_raw(raw.clone()).unwrap();
        assert_eq!(back, m);
        // Trailing bytes are refused, as libcephfs asserts the end.
        let mut bad = raw;
        let mut f = BytesMut::from(&bad.front[..]);
        f.put_u8(0);
        bad.front = f.freeze();
        assert!(MClientSnap::from_raw(bad).is_err());
    }
}
