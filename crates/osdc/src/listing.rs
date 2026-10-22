//! Native namespace-aware pool enumeration. Wire references:
//! `src/osdc/Objecter.h::pg_nls/pg_read`, `src/osd/PrimaryLogPG.cc::do_pg_op`,
//! and `src/osd/osd_types.h::pg_nls_response_template`.
//!
//! Each page reads one PG. Numeric PG order is intentional: the OSD's bitwise
//! continuation stays within that PG; its end marker advances to the next numeric PG.
//! A map epoch change invalidates the entire scan, even if placement did not change.
//! This is not an atomic object snapshot. Bootstrap must exclude concurrent foreign
//! initialization, as required by the persistence contract. The OSD omits its internal
//! hit-set namespace; PGNLS includes all application namespaces.

use std::time::Duration;

use denc::{Bytes, BytesMut, Decode, Encode, Uuid};

use crate::hash::cbits;
use crate::objecter::{Objecter, OpError, OpRequest};
use crate::ops::{ObjectOperation, flags};
use crate::osdmap::map_flags;
use crate::types::{CEPH_NOSNAP, HObject, ObjectLocator, PgId};

/// Namespace selection for pool enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceScope {
    /// All application namespaces, using `LIBRADOS_ALL_NSPACES` (`"\x01"`).
    All,
}

/// Opaque scan continuation. Pass it unchanged to the same pool and Objecter.
/// The handle contains an encoded `hobject_t`, or is empty at the start of a PG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolListCursor {
    pub osdmap_epoch: u32,
    pub pg: u32,
    pub handle: Vec<u8>,
    pool: i64,
    fsid: Uuid,
}

/// One head object. Names and namespaces remain opaque bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolObject {
    pub namespace: Vec<u8>,
    pub name: Vec<u8>,
    pub locator: Vec<u8>,
}

/// A bounded page. Empty objects with `next: Some` means scan progress, not exhaustion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolListPage {
    pub objects: Vec<PoolObject>,
    pub next: Option<PoolListCursor>,
    /// Map snapshot used by the entire scan, including an exhausted final page.
    pub osdmap_epoch: u32,
}

/// Listing never substitutes empty success for a missing map or a failed PG read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PoolListError {
    #[error("OSD map changed; restart the pool scan")]
    MapChanged,
    #[error("no OSD map is installed")]
    NoMap,
    #[error("pool does not exist in the installed map")]
    PoolDoesNotExist,
    #[error("namespace-aware bitwise PG listing is unsupported")]
    Unsupported,
    #[error("invalid pool-list cursor")]
    InvalidCursor,
    #[error("invalid page limits")]
    InvalidLimits,
    #[error("PG listing exceeded the page byte or count bound")]
    SizeLimit,
    #[error("malformed PG listing response")]
    Decode,
    #[error("PG listing failed: {0}")]
    Io(OpError),
}

/// Hard per-turn count bound. Requests above this limit return smaller pages.
pub const MAX_POOL_LIST_ENTRIES: u32 = 1024;
/// Hard per-turn reply/cursor bound, including wire framing and string lengths.
pub const MAX_POOL_LIST_BYTES: usize = 4 * 1024 * 1024;
/// Each PG read has this deadline, independent of the ordinary unbounded MDS op timeout.
pub const POOL_LIST_TIMEOUT: Duration = Duration::from_secs(30);
// v1 response frame (6), empty v4 hobject (39), vector count (4).
const MIN_REPLY_BYTES: usize = 49;

impl Objecter {
    /// Read at most one PG under the cursor's map epoch. `max_entries` must be
    /// nonzero; it is capped at 1024. `max_bytes` must be 49..=4 MiB and bounds
    /// the encoded PG response before decoding or copying its fields. PGNLS has
    /// no server byte-limit field: an oversized OSD response fails with SizeLimit;
    /// it is never truncated or treated as scan completion. Transport frame limits
    /// remain separate. Empty pages may advance past empty PGs.
    ///
    /// Any epoch change, including a PG split, remap, or pool removal, returns
    /// MapChanged. Restart with None; discard previous scan conclusions. Exhaustion
    /// requires a successful end-of-PG response for every PG at one known epoch.
    pub async fn list_pool_page(
        &self,
        pool: i64,
        scope: NamespaceScope,
        cursor: Option<PoolListCursor>,
        max_entries: u32,
        max_bytes: usize,
    ) -> Result<PoolListPage, PoolListError> {
        if max_entries == 0 || !(MIN_REPLY_BYTES..=MAX_POOL_LIST_BYTES).contains(&max_bytes) {
            return Err(PoolListError::InvalidLimits);
        }
        let map = self.osdmap();
        if map.epoch == 0 {
            return Err(PoolListError::NoMap);
        }
        if let Some(c) = &cursor {
            if c.osdmap_epoch != map.epoch || c.fsid != map.fsid {
                return Err(PoolListError::MapChanged);
            }
            if c.pool != pool {
                return Err(PoolListError::InvalidCursor);
            }
        }
        let pi = map.pool(pool).ok_or(PoolListError::PoolDoesNotExist)?;
        if !map.test_flag(map_flags::SORTBITWISE) {
            return Err(PoolListError::Unsupported);
        }
        if pi.pg_num == 0 {
            return Err(PoolListError::Decode);
        }
        let pg = cursor.as_ref().map_or(0, |c| c.pg);
        if pg >= pi.pg_num {
            return Err(PoolListError::InvalidCursor);
        }
        let start = pg_start(pool, pg);
        let end = pg_end(pool, pg, pi.pg_num);
        let handle = match cursor {
            Some(c) if !c.handle.is_empty() => {
                if c.handle.len() > max_bytes {
                    return Err(PoolListError::SizeLimit);
                }
                let mut bytes = Bytes::from(c.handle);
                let h = HObject::decode(&mut bytes).map_err(|_| PoolListError::InvalidCursor)?;
                if !bytes.is_empty() || h < start || h >= end || h.pool != pool {
                    return Err(PoolListError::InvalidCursor);
                }
                h
            }
            _ => start,
        };
        let count = max_entries.min(MAX_POOL_LIST_ENTRIES);
        let mut op = ObjectOperation::new();
        op.pg_nls(u64::from(count), map.epoch, &handle);
        let mut loc = ObjectLocator::new(pool);
        match scope {
            NamespaceScope::All => loc.nspace = Bytes::from_static(b"\x01"),
        }
        // pg_read uses a precalculated raw PG hash, not a hash of the empty oid or
        // the all-namespace selector. ObjectLocator.hash provides that exact routing.
        loc.hash = i64::from(pg);
        let req = OpRequest::new(loc, Bytes::new(), op)
            .with_flags(flags::FAIL_ON_EAGAIN | flags::IGNORE_REDIRECT);
        let reply = tokio::time::timeout(POOL_LIST_TIMEOUT, self.list_pg(req, map.epoch))
            .await
            .map_err(|_| PoolListError::Io(OpError::TimedOut))?;
        if self.epoch() != map.epoch {
            return Err(PoolListError::MapChanged);
        }
        let reply = reply.map_err(list_io)?;
        if reply.epoch != map.epoch {
            return Err(PoolListError::MapChanged);
        }
        if reply.result < 0 {
            return Err(list_io(OpError::Rados(reply.result)));
        }
        if reply.ops.len() != 1 {
            return Err(PoolListError::Io(OpError::InvalidReply));
        }
        if reply.ops[0].rval < 0 {
            return Err(list_io(OpError::Rados(reply.ops[0].rval)));
        }
        if reply.result > 1 {
            return Err(PoolListError::Decode);
        }
        let (next_handle, objects) = decode_reply(reply.out(0), count, max_bytes)?;
        let at_end = next_handle == end;
        if next_handle <= handle || next_handle > end || (reply.result == 1 && !at_end) {
            return Err(PoolListError::Decode);
        }
        let mut previous = handle;
        for (i, object) in objects.iter().enumerate() {
            let key = if object.locator.is_empty() {
                &object.name
            } else {
                &object.locator
            };
            let hash = pi.hash_key(key, &object.namespace);
            if pi.raw_pg_to_pg(PgId::new(pool as u64, hash)).seed != pg {
                return Err(PoolListError::Decode);
            }
            let h = HObject::new(
                object.name.clone(),
                object.locator.clone(),
                CEPH_NOSNAP,
                hash,
                pool,
                object.namespace.clone(),
            );
            if h < previous || h >= next_handle {
                return Err(PoolListError::Decode);
            }
            // PGNLS uses an inclusive lower bound; subsequent objects must be distinct.
            if i > 0 && h == previous {
                return Err(PoolListError::Decode);
            }
            previous = h;
        }
        let next = if at_end && pg == pi.pg_num - 1 {
            None
        } else {
            let mut encoded = BytesMut::new();
            if !at_end {
                next_handle.encode(&mut encoded, 0);
            }
            Some(PoolListCursor {
                osdmap_epoch: map.epoch,
                pg: if at_end { pg + 1 } else { pg },
                handle: encoded.to_vec(),
                pool,
                fsid: map.fsid,
            })
        };
        if self.epoch() != map.epoch {
            return Err(PoolListError::MapChanged);
        }
        Ok(PoolListPage {
            objects,
            next,
            osdmap_epoch: map.epoch,
        })
    }
}

fn list_io(error: OpError) -> PoolListError {
    match error {
        OpError::MapChanged => PoolListError::MapChanged,
        OpError::Rados(-95 | -38) => PoolListError::Unsupported,
        e => PoolListError::Io(e),
    }
}

fn pg_start(pool: i64, pg: u32) -> HObject {
    HObject::new(Bytes::new(), Bytes::new(), 0, pg, pool, Bytes::new())
}

/// `pg_t::get_hobj_end/get_split_bits` (`src/osd/osd_types.cc`).
pub(crate) fn pg_end(pool: i64, pg: u32, pg_num: u32) -> HObject {
    let bits = if pg_num == 1 {
        0
    } else {
        let p = cbits(pg_num);
        let half = 1u64 << (p - 1);
        if u64::from(pg) % half < u64::from(pg_num) % half {
            p
        } else {
            p - 1
        }
    };
    let rev_end = (u64::from(pg.reverse_bits()) | (u64::from(u32::MAX) >> bits)) + 1;
    if rev_end == 1u64 << 32 {
        HObject {
            max: true,
            ..pg_start(0, 0)
        }
    } else {
        HObject::new(
            Bytes::new(),
            Bytes::new(),
            CEPH_NOSNAP,
            (rev_end as u32).reverse_bits(),
            pool,
            Bytes::new(),
        )
    }
}

fn decode_reply(
    mut bytes: Bytes,
    count: u32,
    max_bytes: usize,
) -> Result<(HObject, Vec<PoolObject>), PoolListError> {
    if bytes.len() > max_bytes {
        return Err(PoolListError::SizeLimit);
    }
    let mut frame = denc::decode_start(&mut bytes, 1, "pg_nls_response_t")
        .map_err(|_| PoolListError::Decode)?;
    if frame.v != 1 {
        return Err(PoolListError::Unsupported);
    }
    let h = HObject::decode(&mut frame.body).map_err(|_| PoolListError::Decode)?;
    let n = u32::decode(&mut frame.body).map_err(|_| PoolListError::Decode)?;
    if n > count {
        return Err(PoolListError::SizeLimit);
    }
    let mut objects = Vec::new();
    for _ in 0..n {
        let namespace = Bytes::decode(&mut frame.body).map_err(|_| PoolListError::Decode)?;
        let name = Bytes::decode(&mut frame.body).map_err(|_| PoolListError::Decode)?;
        let locator = Bytes::decode(&mut frame.body).map_err(|_| PoolListError::Decode)?;
        if name.is_empty() {
            return Err(PoolListError::Decode);
        }
        objects.push(PoolObject {
            namespace: namespace.to_vec(),
            name: name.to_vec(),
            locator: locator.to_vec(),
        });
    }
    if !frame.body.is_empty() {
        return Err(PoolListError::Decode);
    }
    // Older OSDs can append a length-prefixed legacy extra_info bufferlist.
    if !bytes.is_empty() {
        Bytes::decode(&mut bytes).map_err(|_| PoolListError::Decode)?;
        if !bytes.is_empty() {
            return Err(PoolListError::Decode);
        }
    }
    Ok((h, objects))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_ceph_namespace_listing_fixture_and_bounds() {
        let text = include_str!("../testdata/pgnls_response3.hex");
        let encoded =
            Bytes::from(hex::decode(text.split_whitespace().collect::<String>()).unwrap());
        let (handle, entries) = decode_reply(encoded.clone(), 3, 4096).unwrap();
        assert_eq!((handle.hash, handle.pool, handle.snap), (4, -1, 3));
        assert_eq!(handle.key.as_ref(), b"key");
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries[1],
            PoolObject {
                namespace: b"n1".to_vec(),
                name: b"n1two".to_vec(),
                locator: b"n1twokey".to_vec()
            }
        );
        assert_eq!(
            decode_reply(encoded.clone(), 2, 4096),
            Err(PoolListError::SizeLimit)
        );
        assert_eq!(
            decode_reply(encoded.clone(), 3, encoded.len() - 1),
            Err(PoolListError::SizeLimit)
        );
        for n in 0..encoded.len() {
            assert!(decode_reply(encoded.slice(..n), 3, 4096).is_err());
        }
        let mut trailing = encoded.to_vec();
        trailing.extend_from_slice(&[0; 4]); // Legacy empty extra_info.
        assert!(decode_reply(Bytes::from(trailing.clone()), 3, 4096).is_ok());
        trailing.push(0);
        assert_eq!(
            decode_reply(Bytes::from(trailing), 3, 4096),
            Err(PoolListError::Decode)
        );
    }

    #[test]
    fn pg_nls_op_layout() {
        let handle = pg_start(7, 2);
        let mut op = ObjectOperation::new();
        op.pg_nls(0x0807060504030201, 0x0c0b0a09, &handle);
        assert_eq!(op.ops[0].op, 0x1505);
        assert_eq!(
            &op.ops[0].args[..12],
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
        );
        assert_eq!(&op.ops[0].args[12..], &[0; 16]);
        assert_eq!(
            HObject::decode(&mut op.ops[0].indata.clone()).unwrap(),
            handle
        );
        assert_eq!(op.flags, flags::PGOP | flags::IGNORE_OVERLAY);
    }

    #[test]
    fn non_power_of_two_pg_ranges_cover_hash_space_once() {
        for count in 1u32..=33 {
            let mut ranges: Vec<_> = (0..count)
                .map(|pg| {
                    (
                        u64::from(pg.reverse_bits()),
                        pg_end(7, pg, count).bitwise_key(),
                    )
                })
                .collect();
            ranges.sort();
            assert_eq!(ranges[0].0, 0);
            for pair in ranges.windows(2) {
                assert_eq!(pair[0].1, pair[1].0);
            }
            assert_eq!(ranges.last().unwrap().1, 1u64 << 32);
        }
    }
}
