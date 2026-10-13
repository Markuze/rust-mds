//! Temporary debug helper: decode captured messages. Remove after use.
use denc::{Message, RawMessage};

fn main() {
    let mut raw = RawMessage::new(24, 6, 1, denc::Bytes::from(std::fs::read(
        "/mnt/workspace/rust-mds/testdata/captures/kernel-v1.msgs/0020-c1-c2s-MClientRequest.front.bin",
    ).unwrap()));
    raw.tid = 3;
    let m = proto::MClientRequest::from_raw(raw).unwrap();
    let a = m.head.args.open();
    println!(
        "REQ tid={} op={:?} flags={:#o} mode={:#o} su={} sc={} osz={} pool={} mask={:#x} owner={}:{}",
        m.tid,
        m.head.op,
        a.flags,
        a.mode,
        a.stripe_unit,
        a.stripe_count,
        a.object_size,
        a.pool,
        a.mask,
        m.head.owner_uid,
        m.head.owner_gid
    );
    println!(
        "  path={:?} path2={:?} releases={} stamp={:?}",
        m.path,
        m.path2,
        m.releases.len(),
        m.stamp
    );

    let arg = std::env::args().nth(1);
    let path = arg.as_deref().unwrap_or(
        "/mnt/workspace/rust-mds/testdata/captures/kernel-v1.msgs/0021-c1-s2c-MClientReply.front.bin",
    );
    let mut raw = RawMessage::new(26, 1, 1, denc::Bytes::from(std::fs::read(path).unwrap()));
    raw.tid = 3;
    let m = proto::MClientReply::from_raw(raw).unwrap();
    println!(
        "REPLY tid={} op={:?} result={} epoch={} dentry={} target={} extra_len={} snapbl={}",
        m.tid,
        m.op,
        m.result,
        m.mdsmap_epoch,
        m.trace.dentry.is_some(),
        m.trace.target.is_some(),
        m.extra.to_bytes().len(),
        m.snapbl.len()
    );
    if let Some(t) = &m.trace.target {
        println!(
            "  target ino={:#x} version={} change_attr={} rbytes={} nfiles={} nsubdirs={} cap_id={} seq={} flags={:#x}",
            t.ino,
            t.version,
            t.change_attr,
            t.rbytes,
            t.nfiles,
            t.nsubdirs,
            t.cap.cap_id,
            t.cap.seq,
            t.cap.flags
        );
    }
    if let Some(d) = &m.trace.dentry {
        println!(
            "  dentry dir={:#x} version={} change_attr={} rbytes={} nfiles={} nsubdirs={} caps={:#x} name={:?}",
            d.dir.ino,
            d.dir.version,
            d.dir.change_attr,
            d.dir.rbytes,
            d.dir.nfiles,
            d.dir.nsubdirs,
            d.dir.cap.caps,
            String::from_utf8_lossy(&d.name)
        );
    }
    match &m.extra {
        proto::ReplyExtra::Create(c) => println!(
            "  extra create ino={:#x} delegated={}",
            c.created_ino,
            c.delegated_inos.len()
        ),
        e => println!("  extra is_none={}", e.is_none()),
    }
}
