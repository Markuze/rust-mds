//! Messages between the MDS and the monitors.
//!
//! | Type | Message | Source |
//! |---|---|---|
//! | 4 | `MMonMap`: `bufferlist monmap` | `src/messages/MMonMap.h` |
//! | 15 | `MMonSubscribe` v3/1: `map<string, {u64 start, u8 flags}>`, `string hostname` | `src/messages/MMonSubscribe.h` |
//! | 16 | `MMonSubscribeAck`: `u32 interval`, `uuid fsid` | `src/messages/MMonSubscribeAck.h` |
//! | 17 | `MAuth`: paxos header, `u32 protocol`, `bufferlist payload`, `u32 monmap_epoch` | `src/messages/MAuth.h` |
//! | 18 | `MAuthReply`: `u32 protocol`, `s32 result`, `u64 global_id`, `bufferlist`, `string msg` | `src/messages/MAuthReply.h` |
//! | 21 | `MMDSMap` v2/1: `uuid fsid`, `u32 epoch`, `bufferlist mdsmap`, `string map_fs_name` | `src/messages/MMDSMap.h` |
//! | 45 | `MFSMap` | `src/messages/MFSMap.h` |
//! | 50 | `MMonCommand` v1/0: paxos header, `uuid fsid`, `vector<string> cmd` | `src/messages/MMonCommand.h` |
//! | 51 | `MMonCommandAck` v1/0: paxos header, `s32 r`, `string rs`, `vector<string> cmd` | `src/messages/MMonCommandAck.h` |
//! | 62 | `MConfig` v1/1: `map<string,string>` | `src/messages/MConfig.h` |
//! | 100 | `MMDSBeacon` v8/6, priority 196 | `src/messages/MMDSBeacon.h` |
//!
//! The paxos header (`PaxosServiceMessage::paxos_encode`) is `u64 version`,
//! `s16 deprecated_session_mon` (-1), `u64 deprecated_session_mon_tid` (0).

use std::collections::BTreeMap;

use bytes::{BufMut, Bytes, BytesMut};
use denc::msg::{PRIO_HIGH, Payload};
use denc::{Decode, Encode, Message, Uuid};

use crate::mdsmap::{CompatSet, MdsState};

pub const CEPH_MSG_MON_MAP: u16 = 4;
pub const CEPH_MSG_MON_SUBSCRIBE: u16 = 15;
pub const CEPH_MSG_MON_SUBSCRIBE_ACK: u16 = 16;
pub const CEPH_MSG_AUTH: u16 = 17;
pub const CEPH_MSG_AUTH_REPLY: u16 = 18;
pub const CEPH_MSG_MON_GET_VERSION: u16 = 19;
pub const CEPH_MSG_MON_GET_VERSION_REPLY: u16 = 20;
pub const CEPH_MSG_MDS_MAP: u16 = 21;
pub const CEPH_MSG_OSD_MAP: u16 = 41;
pub const CEPH_MSG_FS_MAP: u16 = 45;
pub const CEPH_MSG_MON_COMMAND: u16 = 50;
pub const CEPH_MSG_MON_COMMAND_ACK: u16 = 51;
pub const MSG_LOGACK: u16 = 53;
pub const MSG_CONFIG: u16 = 62;
pub const MSG_MDS_BEACON: u16 = 100;
pub const MSG_MGR_MAP: u16 = 0x704;

/// `CEPH_SUBSCRIBE_ONETIME`.
pub const SUBSCRIBE_ONETIME: u8 = 1;

fn paxos_encode(buf: &mut BytesMut, version: u64) {
    buf.put_u64_le(version);
    buf.put_i16_le(-1);
    buf.put_u64_le(0);
}

fn paxos_decode(buf: &mut Bytes) -> denc::Result<u64> {
    let v = u64::decode(buf)?;
    let _session_mon = i16::decode(buf)?;
    let _tid = u64::decode(buf)?;
    Ok(v)
}

/// `MAuth`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MAuth {
    pub protocol: u32,
    pub auth_payload: Bytes,
    pub monmap_epoch: u32,
}

impl Message for MAuth {
    const TYPE: u16 = CEPH_MSG_AUTH;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        paxos_encode(&mut b, 0);
        b.put_u32_le(self.protocol);
        self.auth_payload.encode(&mut b, f);
        b.put_u32_le(self.monmap_epoch);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(_v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        paxos_decode(&mut front)?;
        let protocol = u32::decode(&mut front)?;
        let auth_payload = Bytes::decode(&mut front)?;
        let monmap_epoch = if front.is_empty() {
            0
        } else {
            u32::decode(&mut front)?
        };
        Ok(MAuth {
            protocol,
            auth_payload,
            monmap_epoch,
        })
    }
}

/// `MAuthReply`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MAuthReply {
    pub protocol: u32,
    pub result: i32,
    pub global_id: u64,
    pub result_bl: Bytes,
    pub result_msg: String,
}

impl Message for MAuthReply {
    const TYPE: u16 = CEPH_MSG_AUTH_REPLY;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        b.put_u32_le(self.protocol);
        b.put_i32_le(self.result);
        b.put_u64_le(self.global_id);
        self.result_bl.encode(&mut b, f);
        self.result_msg.encode(&mut b, f);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(_v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        Ok(MAuthReply {
            protocol: u32::decode(&mut front)?,
            result: i32::decode(&mut front)?,
            global_id: u64::decode(&mut front)?,
            result_bl: Bytes::decode(&mut front)?,
            result_msg: String::decode(&mut front)?,
        })
    }
}

/// `MMonSubscribe` (with `SUBSCRIBE2`; the pre-SUBSCRIBE2 form is not produced).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MMonSubscribe {
    /// name -> (start epoch, flags)
    pub what: BTreeMap<String, (u64, u8)>,
    pub hostname: String,
}

impl Message for MMonSubscribe {
    const TYPE: u16 = CEPH_MSG_MON_SUBSCRIBE;
    const HEAD_VERSION: u16 = 3;
    const COMPAT_VERSION: u16 = 1;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        b.put_u32_le(self.what.len() as u32);
        for (k, (start, flags)) in &self.what {
            k.encode(&mut b, f);
            b.put_u64_le(*start);
            b.put_u8(*flags);
        }
        self.hostname.encode(&mut b, f);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        let n = u32::decode(&mut front)?;
        let mut what = BTreeMap::new();
        for _ in 0..n {
            let k = String::decode(&mut front)?;
            let start = u64::decode(&mut front)?;
            let flags = u8::decode(&mut front)?;
            what.insert(k, (start, flags));
        }
        let hostname = if v >= 3 {
            String::decode(&mut front)?
        } else {
            String::new()
        };
        Ok(MMonSubscribe { what, hostname })
    }
}

/// `MMonSubscribeAck`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MMonSubscribeAck {
    pub interval: u32,
    pub fsid: Uuid,
}

impl Message for MMonSubscribeAck {
    const TYPE: u16 = CEPH_MSG_MON_SUBSCRIBE_ACK;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        b.put_u32_le(self.interval);
        self.fsid.encode(&mut b, f);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(_v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        Ok(MMonSubscribeAck {
            interval: u32::decode(&mut front)?,
            fsid: Uuid::decode(&mut front)?,
        })
    }
}

/// `MMonMap`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MMonMap {
    pub monmap: Bytes,
}

impl Message for MMonMap {
    const TYPE: u16 = CEPH_MSG_MON_MAP;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        self.monmap.encode(&mut b, f);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(_v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        Ok(MMonMap {
            monmap: Bytes::decode(&mut front)?,
        })
    }
}

/// `MConfig`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MConfig {
    pub config: BTreeMap<String, String>,
}

impl Message for MConfig {
    const TYPE: u16 = MSG_CONFIG;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 1;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        self.config.encode(&mut b, f);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(_v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        Ok(MConfig {
            config: BTreeMap::decode(&mut front)?,
        })
    }
}

/// `MMDSMap`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MMDSMap {
    pub fsid: Uuid,
    pub epoch: u32,
    /// The encoded `MDSMap` (decode with [`crate::MdsMap`]).
    pub encoded: Bytes,
    pub map_fs_name: String,
}

impl Message for MMDSMap {
    const TYPE: u16 = CEPH_MSG_MDS_MAP;
    const HEAD_VERSION: u16 = 2;
    const COMPAT_VERSION: u16 = 1;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        self.fsid.encode(&mut b, f);
        b.put_u32_le(self.epoch);
        self.encoded.encode(&mut b, f);
        self.map_fs_name.encode(&mut b, f);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        let fsid = Uuid::decode(&mut front)?;
        let epoch = u32::decode(&mut front)?;
        let encoded = Bytes::decode(&mut front)?;
        let map_fs_name = if v >= 2 {
            String::decode(&mut front)?
        } else {
            String::new()
        };
        Ok(MMDSMap {
            fsid,
            epoch,
            encoded,
            map_fs_name,
        })
    }
}

/// `MDSHealthMetric`: `ENCODE_START(1,1)`, `u16 type` (never 0), `u8 sev`, `string message`,
/// `map<string,string> metadata`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MdsHealthMetric {
    pub metric_type: u16,
    pub sev: u8,
    pub message: String,
    pub metadata: BTreeMap<String, String>,
}

impl Encode for MdsHealthMetric {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        assert_ne!(
            self.metric_type, 0,
            "MDS_HEALTH_NULL must never be sent (the mon asserts)"
        );
        denc::encode_versioned(buf, 1, 1, |b| {
            b.put_u16_le(self.metric_type);
            b.put_u8(self.sev);
            self.message.encode(b, f);
            self.metadata.encode(b, f);
        });
    }
}

impl Decode for MdsHealthMetric {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let mut s = denc::decode_start(buf, 1, "MDSHealthMetric")?;
        let b = &mut s.body;
        Ok(MdsHealthMetric {
            metric_type: u16::decode(b)?,
            sev: u8::decode(b)?,
            message: String::decode(b)?,
            metadata: BTreeMap::decode(b)?,
        })
    }
}

/// `MDSHealth`: `ENCODE_START(1,1)` + `vector<MDSHealthMetric>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MdsHealth {
    pub metrics: Vec<MdsHealthMetric>,
}

impl Encode for MdsHealth {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        denc::encode_versioned(buf, 1, 1, |b| self.metrics.encode(b, f));
    }
}

impl Decode for MdsHealth {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let mut s = denc::decode_start(buf, 1, "MDSHealth")?;
        Ok(MdsHealth {
            metrics: Vec::decode(&mut s.body)?,
        })
    }
}

/// `MMDSBeacon` v8, compat 6, priority `CEPH_MSG_PRIO_HIGH`. Payload: paxos header
/// (`version` = last MDSMap epoch seen), `uuid fsid`, `u64 global_id`, `u32 state`, `u64 seq`,
/// `string name`, `s32 standby_for_rank` (-1), `string standby_for_name` (""), `CompatSet`,
/// `MDSHealth`, `map<string,string> sys_info` (only when state is `up:boot`), `u64
/// mds_features`, `s32 standby_for_fscid` (-1), `bool standby_replay` (false), `string fs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MMDSBeacon {
    pub version: u64,
    pub fsid: Uuid,
    pub global_id: u64,
    pub state: MdsState,
    pub seq: u64,
    pub name: String,
    pub compat: CompatSet,
    pub health: MdsHealth,
    pub sys_info: BTreeMap<String, String>,
    pub mds_features: u64,
    pub fs: String,
}

impl Message for MMDSBeacon {
    const TYPE: u16 = MSG_MDS_BEACON;
    const HEAD_VERSION: u16 = 8;
    const COMPAT_VERSION: u16 = 6;
    const PRIORITY: u16 = PRIO_HIGH;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        paxos_encode(&mut b, self.version);
        self.fsid.encode(&mut b, f);
        b.put_u64_le(self.global_id);
        b.put_u32_le(self.state.0 as u32);
        b.put_u64_le(self.seq);
        self.name.encode(&mut b, f);
        b.put_i32_le(-1); // standby_for_rank = MDS_RANK_NONE
        "".encode(&mut b, f); // standby_for_name
        self.compat.encode(&mut b, f);
        self.health.encode(&mut b, f);
        if self.state == MdsState::BOOT {
            self.sys_info.encode(&mut b, f);
        }
        b.put_u64_le(self.mds_features);
        b.put_i32_le(-1); // standby_for_fscid = FS_CLUSTER_ID_NONE
        b.put_u8(0); // standby_replay
        self.fs.encode(&mut b, f);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        let p = &mut front;
        let version = paxos_decode(p)?;
        let fsid = Uuid::decode(p)?;
        let global_id = u64::decode(p)?;
        let state = MdsState(u32::decode(p)? as i32);
        let seq = u64::decode(p)?;
        let name = String::decode(p)?;
        let _standby_for_rank = i32::decode(p)?;
        let _standby_for_name = String::decode(p)?;
        let compat = CompatSet::decode(p)?;
        let health = MdsHealth::decode(p)?;
        let sys_info = if state == MdsState::BOOT {
            BTreeMap::decode(p)?
        } else {
            BTreeMap::new()
        };
        let mds_features = u64::decode(p)?;
        let _standby_for_fscid = i32::decode(p)?;
        if v >= 7 {
            let _standby_replay = bool::decode(p)?;
        }
        let fs = if v >= 8 {
            String::decode(p)?
        } else {
            String::new()
        };
        Ok(MMDSBeacon {
            version,
            fsid,
            global_id,
            state,
            seq,
            name,
            compat,
            health,
            sys_info,
            mds_features,
            fs,
        })
    }
}

/// `MMonCommand` (as `MonClient::start_command` sends it: the command id is
/// the message tid).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MMonCommand {
    pub fsid: Uuid,
    pub cmd: Vec<String>,
}

impl Message for MMonCommand {
    const TYPE: u16 = CEPH_MSG_MON_COMMAND;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        paxos_encode(&mut b, 0);
        self.fsid.encode(&mut b, f);
        self.cmd.encode(&mut b, f);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(_v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        paxos_decode(&mut front)?;
        Ok(MMonCommand {
            fsid: Uuid::decode(&mut front)?,
            cmd: Vec::decode(&mut front)?,
        })
    }
}

/// `MMonCommandAck`; the tid is the command's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MMonCommandAck {
    pub r: i32,
    pub rs: String,
    pub cmd: Vec<String>,
}

impl Message for MMonCommandAck {
    const TYPE: u16 = CEPH_MSG_MON_COMMAND_ACK;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        paxos_encode(&mut b, 0);
        self.r.encode(&mut b, f);
        self.rs.encode(&mut b, f);
        self.cmd.encode(&mut b, f);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(_v: u16, mut front: Bytes, _m: Bytes, _d: Bytes) -> denc::Result<Self> {
        paxos_decode(&mut front)?;
        Ok(MMonCommandAck {
            r: i32::decode(&mut front)?,
            rs: String::decode(&mut front)?,
            cmd: Vec::decode(&mut front)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::{EntityName, EntityType, RawMessage};

    /// The `encode_message` layout ceph-dencoder imports for message types: raw 53-byte
    /// `ceph_msg_header`, 13-byte old footer, then front, middle and data as bufferlists.
    pub fn dencoder_message_bytes(raw: &RawMessage) -> Vec<u8> {
        let mut h = msgr::wire::MsgHeader {
            seq: 0,
            tid: raw.tid,
            msg_type: raw.msg_type,
            priority: raw.priority,
            version: raw.version,
            front_len: raw.front.len() as u32,
            middle_len: raw.middle.len() as u32,
            data_len: raw.data.len() as u32,
            data_off: 0,
            src: EntityName::new(EntityType::MDS, -1),
            compat_version: if raw.compat_version == 0 {
                raw.version
            } else {
                raw.compat_version
            },
            reserved: 0,
            crc: 0,
        };
        let mut out = h.to_bytes().to_vec();
        msgr::wire::MsgFooter {
            flags: 1,
            ..Default::default()
        }
        .encode(false, &mut out);
        for seg in [&raw.front, &raw.middle, &raw.data] {
            out.extend_from_slice(&(seg.len() as u32).to_le_bytes());
            out.extend_from_slice(seg);
        }
        out
    }

    pub fn sample_beacon(state: MdsState) -> MMDSBeacon {
        let mut sys_info = BTreeMap::new();
        sys_info.insert("addr".to_string(), "v1:127.0.0.1:6810/1234".to_string());
        sys_info.insert("hostname".to_string(), "host".to_string());
        MMDSBeacon {
            version: 7,
            fsid: Uuid::parse("dac1ab4b-1c07-4aaa-b7d6-5437ad68143f").unwrap(),
            global_id: 4242,
            state,
            seq: 3,
            name: "rust".into(),
            compat: CompatSet::rust_mds(),
            health: MdsHealth::default(),
            sys_info,
            mds_features: denc::features::ALL,
            fs: "a".into(),
        }
    }

    #[test]
    fn beacon_round_trip() {
        for st in [MdsState::BOOT, MdsState::ACTIVE] {
            let b = sample_beacon(st);
            let raw = b.to_raw(denc::features::ALL);
            assert_eq!((raw.version, raw.compat_version, raw.priority), (8, 6, 196));
            let back = MMDSBeacon::from_raw(raw).unwrap();
            if st == MdsState::BOOT {
                assert_eq!(back, b);
            } else {
                assert!(back.sys_info.is_empty());
            }
        }
    }

    #[test]
    fn mon_command_round_trip() {
        let m = MMonCommand {
            fsid: Uuid::parse("dac1ab4b-1c07-4aaa-b7d6-5437ad68143f").unwrap(),
            cmd: vec![
                "{\"prefix\":\"osd blocklist\", \"blocklistop\":\"add\",\"addr\":\"v1:127.0.0.1:6826/7\"}"
                    .into(),
            ],
        };
        let mut raw = m.to_raw(denc::features::ALL);
        raw.tid = 9;
        assert_eq!(raw.msg_type, CEPH_MSG_MON_COMMAND);
        assert_eq!(MMonCommand::from_raw(raw).unwrap(), m);

        let a = MMonCommandAck {
            r: 0,
            rs: "blocklisted".into(),
            cmd: Vec::new(),
        };
        let mut raw = a.to_raw(denc::features::ALL);
        raw.tid = 9;
        assert_eq!(raw.msg_type, CEPH_MSG_MON_COMMAND_ACK);
        assert_eq!(raw.tid, 9);
        assert_eq!(MMonCommandAck::from_raw(raw).unwrap(), a);
    }

    /// Golden check against Ceph's own decoder, when `ceph-dencoder` is installed: our beacon
    /// bytes must decode to the same fields.
    #[test]
    fn beacon_decodes_with_ceph_dencoder() {
        let Ok(out) = std::process::Command::new("ceph-dencoder")
            .arg("version")
            .output()
        else {
            eprintln!("ceph-dencoder not installed; skipping");
            return;
        };
        if !out.status.success() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("monc-beacon-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("beacon.bin");
        let raw = sample_beacon(MdsState::BOOT).to_raw(denc::features::ALL);
        std::fs::write(&path, dencoder_message_bytes(&raw)).unwrap();
        let out = std::process::Command::new("ceph-dencoder")
            .args(["type", "MMDSBeacon", "import"])
            .arg(&path)
            .args(["decode", "dump_json"])
            .output()
            .unwrap();
        let json = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "dencoder failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        // For messages ceph-dencoder prints `Message::print`. `fs` is the last payload field, so
        // decoding it correctly means every preceding field (compat, health, sys_info, ...) was
        // parsed with the right length.
        assert!(
            json.contains("mdsbeacon(4242/rust up:boot fs=a seq=3 v7)"),
            "{json}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn subscribe_encoding() {
        let mut s = MMonSubscribe {
            hostname: "h".into(),
            ..Default::default()
        };
        s.what.insert("mdsmap".into(), (0, 0));
        let p = s.encode_payload(0);
        // count 1, "mdsmap", le64 start 0, u8 flags 0, hostname "h".
        let expect = [
            "01000000",
            "06000000",
            "6d64736d6170",
            "0000000000000000",
            "00",
            "01000000",
            "68",
        ]
        .concat();
        assert_eq!(hex::encode(&p.front), expect);
        let back =
            MMonSubscribe::decode_payload(3, p.front.freeze(), Bytes::new(), Bytes::new()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn mauth_layout() {
        let m = MAuth {
            protocol: 0,
            auth_payload: Bytes::from_static(b"xy"),
            monmap_epoch: 1,
        };
        let p = m.encode_payload(0).front;
        assert_eq!(
            hex::encode(&p),
            "0000000000000000ffff00000000000000000000000002000000787901000000"
        );
        let r = RawMessage::new(MAuth::TYPE, 1, 0, p.freeze());
        assert_eq!(MAuth::from_raw(r).unwrap(), m);
    }
}
