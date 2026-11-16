//! End-to-end checks of the WAL encoding: engine records survive a
//! frame/write/read/decode round trip and rebuild an identical engine by
//! replay, and a torn or corrupt tail ends the stream where it should.

use bytes::{BufMut, Bytes, BytesMut};

use engine::consts::*;
use engine::journal::{MemJournal, RecordKind};
use engine::types::*;
use engine::{Effect, Engine, EngineConfig, Op, Request};
use store::record;
use store::wal::{FramedRecord, read_segment};

const C1: ClientId = 4100;

/// A small workload across the namespace ops, replayed through the
/// encoding: directories, files with xattrs, a symlink, hard links,
/// renames, unlinks and a stray kept open by caps.
fn workload(e: &mut Engine<MemJournal>) {
    e.open_session(C1);
    let mut tid = 0u64;
    let mut req = |e: &mut Engine<MemJournal>, op: Op| {
        tid += 1;
        let mut r = Request::new(tid, op);
        r.stamp = Timestamp::new((1_700_000_000u64 + tid) as u32, 7);
        let eff = e.handle_request(C1, r);
        drain(e, eff);
    };
    let d1 = e.lookup_name(CEPH_INO_ROOT, b"d1");
    assert!(d1.is_none());
    req(
        e,
        Op::Mkdir {
            path: FilePath::child(CEPH_INO_ROOT, "d1"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let d1 = e.lookup_name(CEPH_INO_ROOT, b"d1").unwrap();
    req(
        e,
        Op::Mkdir {
            path: FilePath::child(CEPH_INO_ROOT, "d2"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let d2 = e.lookup_name(CEPH_INO_ROOT, b"d2").unwrap();
    req(
        e,
        Op::Create {
            path: FilePath::child(d1, "f"),
            flags: CEPH_O_CREAT | CEPH_O_RDWR,
            mode: 0o644,
            layout: Default::default(),
            xattrs: [("user.a".as_bytes().to_vec(), b"1".to_vec())]
                .into_iter()
                .collect(),
            ino: 0,
        },
    );
    let f = e.lookup_name(d1, b"f").unwrap();
    req(
        e,
        Op::Symlink {
            path: FilePath::child(d1, "l"),
            target: b"../d1/f".to_vec(),
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    req(
        e,
        Op::Link {
            path: FilePath::child(d2, "hard"),
            target: FilePath::child(d1, "f"),
        },
    );
    req(
        e,
        Op::Setattr {
            path: FilePath::ino(f),
            args: engine::SetattrArgs {
                mask: CEPH_SETATTR_SIZE | CEPH_SETATTR_MTIME,
                size: 4096,
                ..Default::default()
            },
        },
    );
    req(
        e,
        Op::Rename {
            dst: FilePath::child(d1, "g"),
            src: FilePath::child(d1, "f"),
        },
    );
    req(
        e,
        Op::Setxattr {
            path: FilePath::ino(d1),
            name: b"user.dir".to_vec(),
            value: b"v".to_vec(),
            flags: 0,
        },
    );
    req(
        e,
        Op::Unlink {
            path: FilePath::child(d2, "hard"),
        },
    );
    req(
        e,
        Op::Unlink {
            path: FilePath::child(d1, "g"),
        },
    );
    req(
        e,
        Op::Rmdir {
            path: FilePath::child(CEPH_INO_ROOT, "d2"),
        },
    );
}

/// Consume the effects of a request, acking revokes like a client so
/// parked requests run.
fn drain(e: &mut Engine<MemJournal>, eff: Vec<Effect>) {
    let mut todo: Vec<engine::CapMessage> = eff
        .iter()
        .filter_map(|x| match x {
            Effect::Caps(m) if m.op == CEPH_CAP_OP_REVOKE => Some(m.clone()),
            _ => None,
        })
        .collect();
    while let Some(m) = todo.pop() {
        let ack = engine::CapUpdate {
            op: CEPH_CAP_OP_UPDATE,
            ino: m.ino,
            cap_id: m.cap_id,
            seq: m.seq,
            issue_seq: m.issue_seq,
            mseq: m.mseq,
            caps: m.caps,
            wanted: m.wanted,
            ..Default::default()
        };
        let more = e.handle_cap_update(m.client, ack);
        todo.extend(more.iter().filter_map(|x| match x {
            Effect::Caps(c) if c.op == CEPH_CAP_OP_REVOKE => Some(c.clone()),
            _ => None,
        }));
    }
}

/// A comparable projection of an engine's persistent state: every inode
/// with its core and its directory listing.
type Dump = Vec<(
    Ino,
    Option<engine::inode::InodeCore>,
    Vec<(Vec<u8>, Ino, bool)>,
)>;

fn dump(e: &Engine<MemJournal>) -> Dump {
    let mut out = Vec::new();
    for ino in e.inode_numbers() {
        let core = e.inode(ino).cloned();
        let entries = e.list_dir(ino);
        out.push((ino, core, entries));
    }
    out.sort_by_key(|(ino, _, _)| *ino);
    out
}

fn framed_records(journal: &MemJournal) -> Bytes {
    let mut buf = BytesMut::new();
    for (seq, record) in journal.records() {
        buf.put_slice(&FramedRecord::new(*seq, record).to_bytes());
    }
    buf.freeze()
}

#[test]
fn records_roundtrip_and_replay_rebuilds_the_engine() {
    let mut e = Engine::new(EngineConfig::default(), MemJournal::new());
    workload(&mut e);
    let records: Vec<_> = journal_records(&e).collect();

    // Every produced record survives encode/decode unchanged.
    for (seq, record) in &records {
        let framed = FramedRecord::new(*seq, record);
        let back = framed.decode().expect("record decodes");
        assert_eq!(&back, record, "record {seq} changed over the wire");
    }

    // The framed stream reads back whole, in order, with the right seqs.
    let stream = framed_records(e.journal());
    let segment = read_segment(&stream, 1);
    assert_eq!(segment.len, stream.len(), "the whole segment is valid");
    let got: Vec<(u64, engine::JournalRecord)> = segment
        .records
        .iter()
        .map(|f| (f.seq, f.decode().unwrap()))
        .collect();
    let want: Vec<(u64, engine::JournalRecord)> =
        records.iter().map(|(s, r)| (*s, r.clone())).collect();
    assert_eq!(got, want);

    // Replay from the decoded records rebuilds identical state.
    let decoded: Vec<engine::JournalRecord> = got.into_iter().map(|(_, r)| r).collect();
    let replayed = Engine::replay(EngineConfig::default(), MemJournal::new(), decoded.iter());
    assert_eq!(dump(&replayed), dump(&e));
}

fn journal_records(
    e: &Engine<MemJournal>,
) -> impl Iterator<Item = (u64, engine::JournalRecord)> + '_ {
    e.journal().records().iter().map(|(s, r)| (*s, r.clone()))
}

#[test]
fn torn_tail_ends_the_stream() {
    let mut e = Engine::new(EngineConfig::default(), MemJournal::new());
    workload(&mut e);
    let whole = framed_records(e.journal());
    let n = read_segment(&whole, 1).records.len();
    assert!(n >= 5, "the workload produced enough records");

    // Half a record appended: a crash mid-write.
    let cut = whole.len() - 10;
    let mut torn = BytesMut::new();
    torn.put_slice(&whole[..cut]);
    torn.put_slice(&whole[cut..cut + 5]);
    let torn = torn.freeze();
    let segment = read_segment(&torn, 1);
    assert_eq!(
        segment.records.len(),
        n - 1,
        "the partial record is dropped"
    );
    assert!(segment.len < torn.len(), "the tail is not consumed");

    // A wrong checkpoint (first sequence) rejects everything.
    let none = read_segment(&whole, 2);
    assert!(none.records.is_empty());
}

#[test]
fn corrupt_body_ends_the_stream() {
    let mut e = Engine::new(EngineConfig::default(), MemJournal::new());
    workload(&mut e);
    let mut stream = framed_records(e.journal()).to_vec();
    // Flip one body byte in the second record (header is 24 bytes; the
    // body follows).
    let first_len = 24 + u32::from_le_bytes(stream[0..4].try_into().unwrap()) as usize;
    let body_at = first_len + 24 + 3;
    stream[body_at] ^= 0xff;
    let segment = read_segment(&Bytes::from(stream), 1);
    assert_eq!(segment.records.len(), 1, "the crc stops the reader");
}

#[test]
fn unknown_tail_fields_of_the_same_version_are_ignored() {
    let record = engine::JournalRecord::new(RecordKind::Create);
    let body = record::encode_record(&record);
    // A writer of the same version appended a field we do not know.
    let mut extended = BytesMut::new();
    let s = denc::encode_start(
        &mut extended,
        record::RECORD_VERSION,
        record::RECORD_VERSION,
    );
    extended.put_slice(&body[6..]); // the original frame body
    extended.put_u32_le(0xdeadbeef); // the unknown field
    denc::encode_finish(&mut extended, s);
    let back = record::decode_record(&extended.freeze()).expect("tail is skipped");
    assert_eq!(back, record);
}
