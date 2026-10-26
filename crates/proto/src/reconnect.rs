//! `MClientReconnect` (`CEPH_MSG_CLIENT_RECONNECT`, `src/messages/MClientReconnect.h`): a
//! client's caps, file locks and snap realms, sent to a rank in `up:reconnect` after failover.
//! The payload is in the **data** segment; the front is empty.
//!
//! - **v3** (one message; what the kernel sends): `map<u64 ino, cap_reconnect_t> caps`, then raw
//!   24-byte `ceph_mds_snaprealm_reconnect` records until the end.
//! - **v4/v5** (compat 4; libcephfs, and the kernel above ~2 GiB of state, split over several
//!   messages): `map<u64, cap_reconnect_t> caps`, `vector<snaprealm_reconnect_t> realms`,
//!   v5 `bool more` (another message follows). A part can mix caps and realms
//!   (`src/client/Client.cc`, `Client::send_reconnect`).
//!
//! `cap_reconnect_t` (`src/mds/mdstypes.cc`) = `ENCODE_START(2, 1)` { `string path`, raw
//! `ceph_mds_cap_reconnect` (`u64 cap_id`, `u32 wanted`, `u32 issued`, `u64 snaprealm`,
//! `u64 pathbase`, `u32 flock_len`; 36 bytes), `flock_len` bytes of lock blob, v2 `u64
//! snap_follows` }. `snaprealm_reconnect_t` = `ENCODE_START(1, 1)` { raw `u64 ino, seq, parent` }.
//! Versions 1 and 2 are legacy and refused.

use std::collections::BTreeMap;

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode, Message, Payload, RawMessage, Result};

use crate::msg_type;
use crate::types::{FileLock, decode_counted};
use crate::util::{Ctx, check_raw, get, invalid};

/// The lock blob of a reconnecting cap (`Server::recover_filelocks`): `u32 n` + n raw
/// `ceph_filelock` (fcntl locks), `u32 m` + m raw `ceph_filelock` (flock locks). An empty blob
/// means no locks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlockState {
    pub fcntl: Vec<FileLock>,
    pub flock: Vec<FileLock>,
}

impl FlockState {
    pub fn is_empty(&self) -> bool {
        self.fcntl.is_empty() && self.flock.is_empty()
    }

    pub fn encode(&self) -> Bytes {
        let mut b =
            BytesMut::with_capacity(8 + FileLock::LEN * (self.fcntl.len() + self.flock.len()));
        for set in [&self.fcntl, &self.flock] {
            b.put_u32_le(set.len() as u32);
            for l in set {
                l.encode(&mut b, 0);
            }
        }
        b.freeze()
    }

    /// Parse a lock blob, which must be consumed exactly.
    pub fn decode(mut blob: Bytes) -> Result<Self> {
        if blob.is_empty() {
            return Ok(FlockState::default());
        }
        let b = &mut blob;
        let n = get::<u32>(b, "flock blob fcntl count")? as usize;
        let fcntl = decode_counted::<FileLock>(b, n, "flock blob fcntl")?;
        let n = get::<u32>(b, "flock blob flock count")? as usize;
        let flock = decode_counted::<FileLock>(b, n, "flock blob flock")?;
        if b.has_remaining() {
            return Err(invalid(
                "flock blob",
                format!("{} trailing bytes", b.remaining()),
            ));
        }
        Ok(FlockState { fcntl, flock })
    }
}

/// `cap_reconnect_t`: one cap the client still holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapReconnect {
    /// The primary dentry's name relative to `pathbase` (empty when unknown).
    pub path: Bytes,
    pub cap_id: u64,
    pub wanted: u32,
    pub issued: u32,
    pub snaprealm: u64,
    /// The parent directory inode of `path` (0 when unknown).
    pub pathbase: u64,
    /// Raw lock blob ([`FlockState`]).
    pub flockbl: Bytes,
    /// Oldest pending capsnap's `follows`, or the head snap context seq.
    pub snap_follows: u64,
}

impl CapReconnect {
    /// The cap's file locks.
    pub fn locks(&self) -> Result<FlockState> {
        FlockState::decode(self.flockbl.clone())
    }
}

impl Encode for CapReconnect {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        denc::encode_versioned(buf, 2, 1, |b| {
            self.path.encode(b, f);
            b.put_u64_le(self.cap_id);
            b.put_u32_le(self.wanted);
            b.put_u32_le(self.issued);
            b.put_u64_le(self.snaprealm);
            b.put_u64_le(self.pathbase);
            b.put_u32_le(self.flockbl.len() as u32);
            b.put_slice(&self.flockbl);
            b.put_u64_le(self.snap_follows);
        });
    }
}

impl Decode for CapReconnect {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut fr = denc::decode_start(buf, 2, "cap_reconnect_t")?;
        let b = &mut fr.body;
        let path = get(b, "cap_reconnect_t.path")?;
        denc::need(b, 36, "ceph_mds_cap_reconnect")?;
        let cap_id = b.get_u64_le();
        let wanted = b.get_u32_le();
        let issued = b.get_u32_le();
        let snaprealm = b.get_u64_le();
        let pathbase = b.get_u64_le();
        let flock_len = b.get_u32_le() as usize;
        let flockbl = denc::take(b, flock_len, "cap_reconnect_t.flock")?;
        let snap_follows = if fr.v >= 2 {
            get(b, "cap_reconnect_t.snap_follows")?
        } else {
            0
        };
        Ok(CapReconnect {
            path,
            cap_id,
            wanted,
            issued,
            snaprealm,
            pathbase,
            flockbl,
            snap_follows,
        })
    }
}

/// `ceph_mds_snaprealm_reconnect`: a snap realm the client knows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SnaprealmReconnect {
    pub ino: u64,
    pub seq: u64,
    pub parent: u64,
}

impl SnaprealmReconnect {
    fn put_raw(&self, b: &mut BytesMut) {
        b.put_u64_le(self.ino);
        b.put_u64_le(self.seq);
        b.put_u64_le(self.parent);
    }
    fn get_raw(b: &mut Bytes) -> Result<Self> {
        denc::need(b, 24, "ceph_mds_snaprealm_reconnect")?;
        Ok(SnaprealmReconnect {
            ino: b.get_u64_le(),
            seq: b.get_u64_le(),
            parent: b.get_u64_le(),
        })
    }
}

impl Encode for SnaprealmReconnect {
    fn encode(&self, buf: &mut BytesMut, _f: u64) {
        denc::encode_versioned(buf, 1, 1, |b| self.put_raw(b));
    }
}

impl Decode for SnaprealmReconnect {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut fr = denc::decode_start(buf, 1, "snaprealm_reconnect_t")?;
        Self::get_raw(&mut fr.body)
    }
}

/// `MClientReconnect`. See the module documentation for the wire format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MClientReconnect {
    pub caps: BTreeMap<u64, CapReconnect>,
    pub realms: Vec<SnaprealmReconnect>,
    /// v5: another part follows.
    pub more: bool,
    /// The header version this message was decoded from, and the one [`MClientReconnect::to_raw`]
    /// encodes with (3, 4 or 5).
    pub version: u16,
}

impl Default for MClientReconnect {
    fn default() -> Self {
        MClientReconnect {
            caps: BTreeMap::new(),
            realms: Vec::new(),
            more: false,
            version: Self::HEAD_VERSION,
        }
    }
}

impl Message for MClientReconnect {
    const TYPE: u16 = msg_type::CLIENT_RECONNECT;
    const HEAD_VERSION: u16 = 5;
    const COMPAT_VERSION: u16 = 4;

    /// Encodes at `self.version` (clients send reconnects; this is for tests).
    fn encode_payload(&self, f: u64) -> Payload {
        let mut d = BytesMut::new();
        d.put_u32_le(self.caps.len() as u32);
        for (ino, c) in &self.caps {
            d.put_u64_le(*ino);
            c.encode(&mut d, f);
        }
        if self.version >= 4 {
            d.put_u32_le(self.realms.len() as u32);
            for r in &self.realms {
                r.encode(&mut d, f);
            }
            if self.version >= 5 {
                d.put_u8(u8::from(self.more));
            }
        } else {
            for r in &self.realms {
                r.put_raw(&mut d);
            }
        }
        Payload {
            front: BytesMut::new(),
            middle: Bytes::new(),
            data: d.freeze(),
        }
    }

    fn decode_payload(
        version: u16,
        _front: Bytes,
        _middle: Bytes,
        mut data: Bytes,
    ) -> Result<Self> {
        if version < 3 {
            return Err(invalid(
                "MClientReconnect",
                format!("legacy version {version} is not supported"),
            ));
        }
        let p = &mut data;
        let n = get::<u32>(p, "MClientReconnect.caps")?;
        let mut caps = BTreeMap::new();
        for _ in 0..n {
            let ino = get::<u64>(p, "MClientReconnect.caps ino")?;
            caps.insert(ino, CapReconnect::decode(p).ctx("MClientReconnect.caps")?);
        }
        let mut m = MClientReconnect {
            caps,
            version,
            ..Default::default()
        };
        if version >= 4 {
            let n = get::<u32>(p, "MClientReconnect.realms")? as usize;
            m.realms = decode_counted(p, n, "MClientReconnect.realms")?;
            if version >= 5 {
                m.more = get(p, "MClientReconnect.more")?;
            }
        } else {
            while p.has_remaining() {
                m.realms
                    .push(SnaprealmReconnect::get_raw(p).ctx("MClientReconnect v3 realms")?);
            }
        }
        Ok(m)
    }

    fn to_raw(&self, features: u64) -> RawMessage {
        let p = self.encode_payload(features);
        let compat = if self.version <= 3 {
            0
        } else {
            Self::COMPAT_VERSION
        };
        let mut raw = RawMessage::new(Self::TYPE, self.version, compat, p.front.freeze());
        raw.data = p.data;
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
    use crate::types::filelock;

    fn sample(version: u16) -> MClientReconnect {
        let locks = FlockState {
            fcntl: vec![FileLock {
                start: 0,
                length: 10,
                client: 0,
                owner: 1 << 63 | 5,
                pid: 42,
                lock_type: filelock::EXCL,
            }],
            flock: vec![],
        };
        let mut caps = BTreeMap::new();
        caps.insert(
            0x100_0000_0001,
            CapReconnect {
                path: Bytes::from_static(b"f"),
                cap_id: 3,
                wanted: 0x55,
                issued: 0x555,
                snaprealm: 1,
                pathbase: 1,
                flockbl: locks.encode(),
                snap_follows: 0,
            },
        );
        caps.insert(
            1,
            CapReconnect {
                cap_id: 1,
                issued: 0x155,
                snaprealm: 1,
                flockbl: FlockState::default().encode(),
                ..Default::default()
            },
        );
        MClientReconnect {
            caps,
            realms: vec![SnaprealmReconnect {
                ino: 1,
                seq: 1,
                parent: 0,
            }],
            more: false,
            version,
        }
    }

    #[test]
    fn v3_kernel_layout() {
        let m = sample(3);
        let raw = m.to_raw(0);
        assert_eq!((raw.version, raw.compat_version), (3, 0));
        assert!(raw.front.is_empty());
        let d = &raw.data;
        assert_eq!(&d[0..4], &2u32.to_le_bytes());
        assert_eq!(&d[4..12], &1u64.to_le_bytes()); // map order: ino 1 first
        assert_eq!(&d[12..14], &[2, 1]); // cap_reconnect_t v2 compat 1
        // Raw realm record at the end.
        assert_eq!(&d[d.len() - 24..d.len() - 16], &1u64.to_le_bytes());
        let back = MClientReconnect::from_raw(raw).unwrap();
        assert_eq!(back, m);
        let c = &back.caps[&0x100_0000_0001];
        assert_eq!(c.locks().unwrap().fcntl[0].pid, 42);
        assert_eq!(c.flockbl.len(), 8 + 41);
        assert!(back.caps[&1].locks().unwrap().is_empty());
    }

    #[test]
    fn v5_parts() {
        let mut m = sample(5);
        m.more = true;
        let raw = m.to_raw(0);
        assert_eq!((raw.version, raw.compat_version), (5, 4));
        assert_eq!(raw.data[raw.data.len() - 1], 1);
        let back = MClientReconnect::from_raw(raw).unwrap();
        assert_eq!(back, m);
        // A realms-only part.
        let part = MClientReconnect {
            caps: BTreeMap::new(),
            realms: vec![SnaprealmReconnect {
                ino: 0x1000,
                seq: 3,
                parent: 1,
            }],
            more: false,
            version: 5,
        };
        let raw = part.to_raw(0);
        assert_eq!(hex::encode(&raw.data[..10]), "00000000010000000101");
        assert_eq!(MClientReconnect::from_raw(raw).unwrap(), part);
    }

    #[test]
    fn malformed() {
        // Legacy versions are refused.
        let raw = RawMessage {
            version: 2,
            ..sample(3).to_raw(0)
        };
        assert!(MClientReconnect::from_raw(raw).is_err());
        // v3 realms that are not a whole number of records.
        let raw = sample(3).to_raw(0);
        let cut = RawMessage {
            data: raw.data.slice(..raw.data.len() - 1),
            ..raw
        };
        assert!(MClientReconnect::from_raw(cut).is_err());
        // A bad lock blob is reported when parsed, not when decoding the message.
        let mut m = sample(5);
        m.caps.get_mut(&1).unwrap().flockbl = Bytes::from_static(&[1, 0, 0, 0]);
        let back = MClientReconnect::from_raw(m.to_raw(0)).unwrap();
        assert!(back.caps[&1].locks().is_err());
    }
}
