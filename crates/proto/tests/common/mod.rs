//! Test helpers: a minimal pcap / pcapng reader and TCP stream reassembly, enough to pull
//! msgr1 conversations out of the dev cluster's loopback captures.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

/// One TCP direction: (source address, source port, destination address, destination port).
pub type FlowKey = (IpAddr, u16, IpAddr, u16);

#[derive(Default)]
struct Flow {
    isn: Option<u32>,
    syn: bool,
    syn_ack: bool,
    first_ts: f64,
    /// relative sequence -> payload
    segs: BTreeMap<u64, Vec<u8>>,
}

/// A reassembled TCP connection.
pub struct Connection {
    pub client: (IpAddr, u16),
    pub server: (IpAddr, u16),
    pub client_to_server: Vec<u8>,
    pub server_to_client: Vec<u8>,
    pub start_ts: f64,
}

fn u16be(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}
fn u32be(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes(b[o..o + 4].try_into().unwrap())
}

/// Read every packet of a pcap or pcapng file: (timestamp, link type, frame bytes).
pub fn read_packets(path: &Path) -> Vec<(f64, u32, Vec<u8>)> {
    let b = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut out = Vec::new();
    if b.len() < 24 {
        return out;
    }
    let magic = u32::from_le_bytes(b[0..4].try_into().unwrap());
    if magic == 0x0A0D_0D0A {
        return read_pcapng(&b);
    }
    let (le, nanos) = match magic {
        0xa1b2_c3d4 => (true, false),
        0xa1b2_3c4d => (true, true),
        0xd4c3_b2a1 => (false, false),
        0x4d3c_b2a1 => (false, true),
        m => panic!("{}: not a pcap file (magic {m:#x})", path.display()),
    };
    let rd32 = |o: usize| {
        let a: [u8; 4] = b[o..o + 4].try_into().unwrap();
        if le {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        }
    };
    let linktype = rd32(20) & 0x0fff_ffff;
    let mut off = 24;
    while off + 16 <= b.len() {
        let ts = rd32(off) as f64 + rd32(off + 4) as f64 / if nanos { 1e9 } else { 1e6 };
        let incl = rd32(off + 8) as usize;
        off += 16;
        if off + incl > b.len() {
            break;
        }
        out.push((ts, linktype, b[off..off + incl].to_vec()));
        off += incl;
    }
    out
}

fn read_pcapng(b: &[u8]) -> Vec<(f64, u32, Vec<u8>)> {
    let mut out = Vec::new();
    let mut linktypes = Vec::new();
    let mut tsres = Vec::new();
    let mut off = 0;
    let u32le = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    while off + 12 <= b.len() {
        let btype = u32le(off);
        let blen = u32le(off + 4) as usize;
        if blen < 12 || off + blen > b.len() {
            break;
        }
        match btype {
            0x0A0D_0D0A => {
                linktypes.clear();
                tsres.clear();
            }
            1 => {
                linktypes.push(u16::from_le_bytes([b[off + 8], b[off + 9]]) as u32);
                tsres.push(1e6);
            }
            6 => {
                let ifid = u32le(off + 8) as usize;
                let ts = ((u32le(off + 12) as u64) << 32 | u32le(off + 16) as u64) as f64
                    / tsres.get(ifid).copied().unwrap_or(1e6);
                let caplen = u32le(off + 20) as usize;
                let data = &b[off + 28..(off + 28 + caplen).min(off + blen)];
                out.push((ts, linktypes.get(ifid).copied().unwrap_or(1), data.to_vec()));
            }
            _ => {}
        }
        off += blen;
    }
    out
}

/// Strip the link layer; return the IP packet.
fn ip_payload(linktype: u32, frame: &[u8]) -> Option<&[u8]> {
    let (off, proto) = match linktype {
        1 => {
            let mut off = 12;
            let mut et = u16be(frame, off);
            while et == 0x8100 || et == 0x88a8 {
                off += 4;
                et = u16be(frame, off);
            }
            (off + 2, et)
        }
        113 => (16, u16be(frame, 14)),
        276 => (20, u16be(frame, 0)),
        0 => (
            4,
            if frame.get(0..4)? == [2, 0, 0, 0] {
                0x0800
            } else {
                0x86dd
            },
        ),
        101 | 12 | 14 => (
            0,
            if frame.first()? >> 4 == 4 {
                0x0800
            } else {
                0x86dd
            },
        ),
        _ => return None,
    };
    if proto != 0x0800 && proto != 0x86dd {
        return None;
    }
    frame.get(off..)
}

/// Group TCP payloads into connections.
pub fn tcp_connections(path: &Path) -> Vec<Connection> {
    let mut flows: BTreeMap<FlowKey, Flow> = BTreeMap::new();
    for (ts, lt, frame) in read_packets(path) {
        let Some(ip) = ip_payload(lt, &frame) else {
            continue;
        };
        if ip.is_empty() {
            continue;
        }
        let (src, dst, tcp) = match ip[0] >> 4 {
            4 => {
                if ip.len() < 20 || ip[9] != 6 {
                    continue;
                }
                let ihl = (ip[0] & 15) as usize * 4;
                let total = (u16be(ip, 2) as usize).min(ip.len());
                let src = IpAddr::from(<[u8; 4]>::try_from(&ip[12..16]).unwrap());
                let dst = IpAddr::from(<[u8; 4]>::try_from(&ip[16..20]).unwrap());
                (src, dst, &ip[ihl..total])
            }
            6 => {
                if ip.len() < 40 || ip[6] != 6 {
                    continue;
                }
                let plen = u16be(ip, 4) as usize;
                let src = IpAddr::from(<[u8; 16]>::try_from(&ip[8..24]).unwrap());
                let dst = IpAddr::from(<[u8; 16]>::try_from(&ip[24..40]).unwrap());
                (src, dst, &ip[40..(40 + plen).min(ip.len())])
            }
            _ => continue,
        };
        if tcp.len() < 20 {
            continue;
        }
        let sport = u16be(tcp, 0);
        let dport = u16be(tcp, 2);
        let seq = u32be(tcp, 4);
        let doff = (tcp[12] >> 4) as usize * 4;
        let flags = tcp[13];
        let (syn, ack) = (flags & 0x02 != 0, flags & 0x10 != 0);
        let f = flows.entry((src, sport, dst, dport)).or_default();
        if f.isn.is_none() || syn {
            if f.isn.is_none() {
                f.first_ts = ts;
            }
            f.isn = Some(if syn { seq.wrapping_add(1) } else { seq });
            f.syn |= syn && !ack;
            f.syn_ack |= syn && ack;
        }
        let payload = tcp.get(doff..).unwrap_or(&[]);
        if !payload.is_empty() {
            let rel = seq.wrapping_sub(f.isn.unwrap()) as u64;
            f.segs.entry(rel).or_insert_with(|| payload.to_vec());
        }
    }
    let assemble = |f: &Flow| {
        let mut data: Vec<u8> = Vec::new();
        for (rel, p) in &f.segs {
            let rel = *rel as usize;
            if rel > data.len() {
                break; // a gap: stop at the hole
            }
            let skip = data.len() - rel;
            if skip < p.len() {
                data.extend_from_slice(&p[skip..]);
            }
        }
        data
    };
    let mut out = Vec::new();
    let keys: Vec<FlowKey> = flows.keys().copied().collect();
    for k in keys {
        let f = &flows[&k];
        let rev = (k.2, k.3, k.0, k.1);
        // Report each connection once, from the side that opened it (sent the bare SYN), or
        // from the lower port when the capture missed the handshake.
        let is_client =
            f.syn || (!f.syn_ack && !flows.get(&rev).is_some_and(|r| r.syn) && k.1 > k.3);
        if !is_client {
            continue;
        }
        let empty = Flow::default();
        let r = flows.get(&rev).unwrap_or(&empty);
        out.push(Connection {
            client: (k.0, k.1),
            server: (k.2, k.3),
            client_to_server: assemble(f),
            server_to_client: assemble(r),
            start_ts: f.first_ts,
        });
    }
    out.sort_by(|a, b| a.start_ts.partial_cmp(&b.start_ts).unwrap());
    out
}

/// The directory with the dev cluster's captures (`PROTO_CAPTURES` overrides it).
pub fn captures_dir() -> PathBuf {
    std::env::var_os("PROTO_CAPTURES")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/captures")
        })
}

/// `tests/data/<name>.msgs`: the client<->MDS messages of a capture, so the golden checks run
/// without the dev cluster. Format: records of `u8 direction` (0 = client to MDS, 1 = MDS to
/// client), the raw 53-byte `ceph_msg_header`, then front, middle and data as `u32 len` + bytes.
pub fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(format!("{name}.msgs"))
}

pub fn write_corpus(path: &Path, convs: &[proto::wire::Msgr1Conversation]) {
    let mut out = bytes::BytesMut::new();
    use bytes::BufMut;
    for conv in convs {
        for (dir, msgs) in [(0u8, &conv.from_client), (1u8, &conv.from_server)] {
            for m in msgs {
                out.put_u8(dir);
                m.header.encode(&mut out);
                for seg in [&m.raw.front, &m.raw.middle, &m.raw.data] {
                    out.put_u32_le(seg.len() as u32);
                    out.put_slice(seg);
                }
            }
        }
    }
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, &out).unwrap();
}

pub fn read_corpus(path: &Path) -> proto::wire::Msgr1Conversation {
    let b = std::fs::read(path).unwrap();
    let mut conv = proto::wire::Msgr1Conversation::default();
    let mut off = 0;
    while off < b.len() {
        let dir = b[off];
        off += 1;
        let header = proto::wire::MsgHeader::decode(&b[off..]).unwrap();
        off += proto::wire::MsgHeader::LEN;
        let mut segs = Vec::new();
        for _ in 0..3 {
            let len = u32::from_le_bytes(b[off..off + 4].try_into().unwrap()) as usize;
            off += 4;
            segs.push(bytes::Bytes::copy_from_slice(&b[off..off + len]));
            off += len;
        }
        let raw = header.to_raw(segs[0].clone(), segs[1].clone(), segs[2].clone());
        let m = proto::wire::Msgr1Message { header, raw };
        if dir == 0 {
            conv.from_client.push(m)
        } else {
            conv.from_server.push(m)
        }
    }
    conv
}

/// msgr2.1 (`rev1`) crc-mode stream: extract the MESSAGE frames of one direction.
///
/// Layout (`src/msg/async/frames_v2.cc`, `FrameAssembler::asm_crc_rev1`): the banner `ceph
/// v2\n`, a `u16` length and the features; then frames made of a 32-byte preamble (`u8 tag`,
/// `u8 num_segments`, four `u32 len` + `u16 align` pairs, `u8 flags`, `u8 reserved`, `u32
/// crc`), segment 0 followed by its `u32` crc when non-empty, and, with more than one segment,
/// the other segments and a 13-byte epilogue. A MESSAGE frame (tag 17) has the segments
/// `ceph_msg_header2` (41 bytes), front, middle and data (trailing empty ones are left out).
pub fn msgr2_messages(stream: &[u8]) -> Result<Vec<proto::wire::Msgr1Message>, String> {
    const BANNER2: &[u8] = b"ceph v2\n";
    if !stream.starts_with(BANNER2) {
        return Err("no msgr2 banner".into());
    }
    let mut off = BANNER2.len();
    let blen = u16::from_le_bytes(
        stream
            .get(off..off + 2)
            .ok_or("short banner")?
            .try_into()
            .unwrap(),
    ) as usize;
    off += 2 + blen;
    let mut out = Vec::new();
    while off + 32 <= stream.len() {
        let pre = &stream[off..off + 32];
        let tag = pre[0];
        let nseg = pre[1] as usize;
        if nseg == 0 || nseg > 4 {
            return Err(format!("bad preamble at {off}: {} segments", nseg));
        }
        let lens: Vec<usize> = (0..nseg)
            .map(|i| u32::from_le_bytes(pre[2 + 6 * i..6 + 6 * i].try_into().unwrap()) as usize)
            .collect();
        off += 32;
        let mut segs: Vec<&[u8]> = Vec::new();
        let total = lens.iter().sum::<usize>()
            + if lens[0] > 0 { 4 } else { 0 }
            + if nseg > 1 { 13 } else { 0 };
        if off + total > stream.len() {
            break; // cut off
        }
        segs.push(&stream[off..off + lens[0]]);
        off += lens[0];
        if lens[0] > 0 {
            off += 4;
        }
        for len in &lens[1..] {
            segs.push(&stream[off..off + len]);
            off += len;
        }
        if nseg > 1 {
            off += 13;
        }
        if tag == 17 {
            let h = segs[0];
            if h.len() < 41 {
                return Err("short ceph_msg_header2".into());
            }
            let u16_at = |o: usize| u16::from_le_bytes([h[o], h[o + 1]]);
            let u64_at = |o: usize| u64::from_le_bytes(h[o..o + 8].try_into().unwrap());
            let seg = |i: usize| bytes::Bytes::copy_from_slice(segs.get(i).copied().unwrap_or(&[]));
            let header = proto::wire::MsgHeader {
                seq: u64_at(0),
                tid: u64_at(8),
                msg_type: u16_at(16),
                priority: u16_at(18),
                version: u16_at(20),
                front_len: segs.get(1).map_or(0, |s| s.len() as u32),
                middle_len: segs.get(2).map_or(0, |s| s.len() as u32),
                data_len: segs.get(3).map_or(0, |s| s.len() as u32),
                data_off: u16_at(26),
                src_type: 0,
                src_num: 0,
                compat_version: u16_at(37),
                reserved: 0,
                crc: 0,
            };
            let raw = header.to_raw(seg(1), seg(2), seg(3));
            out.push(proto::wire::Msgr1Message { header, raw });
        }
    }
    Ok(out)
}
