//! Byte layouts from `src/messages/MMonGetVersion{,Reply}.h`.
use bytes::Bytes;
use denc::Message;
use monc::messages::{MMonGetVersion, MMonGetVersionReply};

#[test]
fn version_payload_layouts() {
    let q = MMonGetVersion {
        handle: 0x0807060504030201,
        what: "osdmap".into(),
    };
    let raw = q.to_raw(0);
    assert_eq!(
        raw.front.as_ref(),
        b"\x01\x02\x03\x04\x05\x06\x07\x08\x06\0\0\0osdmap"
    );
    assert_eq!(raw.tid, 0);
    assert_eq!(MMonGetVersion::from_raw(raw).unwrap(), q);
    let r = MMonGetVersionReply {
        handle: 1,
        version: 2,
        oldest_version: 3,
    };
    let raw = r.to_raw(0);
    assert_eq!(
        raw.front.as_ref(),
        b"\x01\0\0\0\0\0\0\0\x02\0\0\0\0\0\0\0\x03\0\0\0\0\0\0\0"
    );
    assert_eq!(MMonGetVersionReply::from_raw(raw.clone()).unwrap(), r);
    let mut old = raw;
    old.version = 1;
    old.front = old.front.slice(..16);
    assert_eq!(
        MMonGetVersionReply::from_raw(old).unwrap().oldest_version,
        0
    );
    assert!(
        MMonGetVersionReply::decode_payload(2, Bytes::new(), Bytes::new(), Bytes::new()).is_err()
    );
}
