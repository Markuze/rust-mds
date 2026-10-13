//! The RADOS work behind the engine's daemon-work effects (`Filer`, the
//! `PurgeQueue` executor and `RecoveryQueue` of the C++ MDS).
//!
//! Object naming and striping are shared with the clients that write file
//! data, so deletion must follow the same naming: `<ino hex>.<objno %08x>`
//! (`file_object_t`, `src/include/object.h`), with the object count of
//! `Striper::get_num_objects`.
//!
//! - `PurgeInode` deletes objects 0..N, N covering the largest size a client
//!   may have written (`PurgeQueue::_execute_item`, `-ENOENT` ignored);
//! - `TruncateData` sends `CEPH_OSD_OP_TRIMTRUNC` (object-local truncate
//!   offset, `truncate_seq`) to every object overlapping
//!   `[size, from)` (`MDCache::_truncate_inode` through `Filer::truncate`,
//!   `Striper::object_truncate_size`);
//! - `RecoverFileSize` probes objects backwards from `max_size`, also asking
//!   for the mtime (`Filer::probe`).
//!
//! All MDS OSD ops carry `FULL_FORCE` (the `Objecter` adds it because
//! `honor_full` is false), so purge proceeds on full pools.

use bytes::Bytes;

use osdc::objecter::{Objecter, OpError, OpRequest};
use osdc::ops::ObjectOperation;
use osdc::types::{ObjectLocator, SnapContext};

use engine::types::FileLayout;

/// The data object `objno` of `ino` (`file_object_t::c_str`: `%llx.%08llx`).
pub fn object_name(ino: u64, objno: u64) -> String {
    format!("{ino:x}.{objno:08x}")
}

/// `Striper::get_num_objects`: how many objects a file of `size` bytes
/// covers under `layout`.
pub fn num_objects(layout: &FileLayout, size: u64) -> u64 {
    let su = layout.stripe_unit.max(1) as u64;
    let sc = layout.stripe_count.max(1) as u64;
    let period = su * sc;
    let num_periods = size.div_ceil(period);
    let remainder_bytes = size % period;
    let mut remainder_objs = 0;
    if remainder_bytes > 0 && remainder_bytes < sc * su {
        remainder_objs = sc - remainder_bytes.div_ceil(su);
    }
    num_periods * sc - remainder_objs
}

/// `Striper::object_truncate_size` for the object that the truncate to
/// `trunc_size` ends in (`None` for objects before it, `Some(0)` for objects
/// after it: `TRIMTRUNC` to 0 empties them but keeps object 0, which carries
/// the backtrace).
fn object_truncate_size(layout: &FileLayout, trunc_size: u64, objno: u64) -> Option<u64> {
    let su = layout.stripe_unit.max(1) as u64;
    let object_size = layout.object_size.max(1) as u64;
    let stripes_per_object = (object_size / su).max(1);
    let trunc_stripeno = trunc_size / su;
    let trunc_objno = trunc_stripeno / stripes_per_object;
    if objno < trunc_objno {
        None
    } else if objno == trunc_objno {
        // The striping offset of the truncation point within the object.
        Some((trunc_stripeno % stripes_per_object) * su + trunc_size % su)
    } else {
        Some(0)
    }
}

/// The stripes covering `[off, off + len)` map to these object numbers, in
/// order (`Striper::file_to_extents` object numbering, first to last).
fn objects_overlapping(layout: &FileLayout, off: u64, len: u64) -> Vec<u64> {
    let su = layout.stripe_unit.max(1) as u64;
    let stripes_per_object = (layout.object_size.max(1) as u64 / su).max(1);
    let first = off / su / stripes_per_object;
    let last = off.saturating_add(len.saturating_sub(1)) / su / stripes_per_object;
    (first..=last).collect()
}

fn oloc_for(layout: &FileLayout) -> ObjectLocator {
    let mut oloc = ObjectLocator::new(layout.pool_id);
    oloc.nspace = Bytes::from(layout.pool_ns.clone());
    oloc
}

fn snapc_head() -> SnapContext {
    SnapContext::default()
}

/// Delete the data objects of a purged inode (`PurgeInode`): objects 0..N in
/// the layout pool and namespace, `-ENOENT` ignored, every other error
/// logged and counted as done (like the C++ executor).
pub async fn purge_inode(objecter: &Objecter, ino: u64, layout: &FileLayout, size: u64) {
    let oloc = oloc_for(layout);
    for objno in 0..num_objects(layout, size) {
        let oid = object_name(ino, objno);
        match objecter
            .remove(oloc.clone(), oid.clone(), snapc_head())
            .await
        {
            Ok(_) | Err(OpError::Rados(-2)) => {}
            Err(e) => {
                tracing::warn!(ino, objno, %e, "purge: object remove failed");
            }
        }
    }
}

/// Truncate the data objects of an inode (`TruncateData`): `TRIMTRUNC` to the
/// object-local size with `truncate_seq`, on every object overlapping
/// `[size, from)`.
pub async fn truncate_data(
    objecter: &Objecter,
    ino: u64,
    layout: &FileLayout,
    truncate_seq: u32,
    size: u64,
    from: u64,
) {
    if from <= size {
        return;
    }
    let oloc = oloc_for(layout);
    for objno in objects_overlapping(layout, size, from - size) {
        let Some(local) = object_truncate_size(layout, size, objno) else {
            continue;
        };
        let mut op = ObjectOperation::new();
        op.trimtrunc(truncate_seq, local);
        if let Err(e) = objecter
            .operate(OpRequest::new(oloc.clone(), object_name(ino, objno), op))
            .await
        {
            tracing::warn!(ino, objno, %e, "truncate: TRIMTRUNC failed");
        }
    }
}

/// The recovered size and mtime of a file whose writer was lost
/// (`RecoverFileSize`): probe objects backwards from `max_size`
/// (`Filer::probe`); `None` size 0 / no object found.
pub async fn recover_file_size(
    objecter: &Objecter,
    ino: u64,
    layout: &FileLayout,
    max_size: u64,
) -> Option<(u64, denc::UTime)> {
    let oloc = oloc_for(layout);
    let object_size = layout.object_size.max(1) as u64;
    let last = max_size.saturating_sub(1) / object_size;
    for objno in (0..=last).rev() {
        match objecter.stat(oloc.clone(), object_name(ino, objno)).await {
            Ok((n, mtime)) => {
                return Some((objno * object_size + n.min(object_size), mtime));
            }
            Err(OpError::Rados(-2)) => continue,
            Err(e) => {
                tracing::warn!(ino, objno, %e, "recover: stat failed");
                return None;
            }
        }
    }
    None
}

/// Run a daemon-work effect on the tokio runtime, logging its outcome. Every
/// result counts as done (the C++ PurgeQueue expires the item); only
/// `-EBLOCKLISTED` is fatal, which the Objecter reports as `Fenced`.
pub fn spawn_purge(objecter: Objecter, ino: u64, layout: FileLayout, size: u64) {
    tokio::spawn(async move {
        purge_inode(&objecter, ino, &layout, size).await;
        tracing::debug!(ino, "purge done");
    });
}

pub fn spawn_truncate(
    objecter: Objecter,
    ino: u64,
    layout: FileLayout,
    truncate_seq: u32,
    size: u64,
    from: u64,
) {
    tokio::spawn(async move {
        truncate_data(&objecter, ino, &layout, truncate_seq, size, from).await;
        tracing::debug!(ino, "truncate done");
    });
}

pub fn spawn_recover(objecter: Objecter, ino: u64, layout: FileLayout, max_size: u64) {
    tokio::spawn(async move {
        match recover_file_size(&objecter, ino, &layout, max_size).await {
            // No engine callback yet (the recovered size cannot be applied);
            // logged until 5.3 persistence adds one.
            Some((size, mtime)) => {
                tracing::info!(ino, size, ?mtime, "recovered file size")
            }
            None => tracing::info!(ino, "file size recovery found no objects"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> FileLayout {
        FileLayout {
            stripe_unit: 4 << 20,
            stripe_count: 1,
            object_size: 4 << 20,
            pool_id: 3,
            pool_ns: String::new(),
        }
    }

    #[test]
    fn names_match_file_object_t() {
        assert_eq!(object_name(0x1000, 0), "1000.00000000");
        assert_eq!(object_name(0x123456789abc, 42), "123456789abc.0000002a");
    }

    #[test]
    fn num_objects_covers_ceil_and_stripes() {
        let l = layout();
        assert_eq!(num_objects(&l, 0), 0);
        assert_eq!(num_objects(&l, 1), 1);
        assert_eq!(num_objects(&l, 4 << 20), 1);
        assert_eq!(num_objects(&l, (4 << 20) + 1), 2);
        // A striped layout: one object per period (`object_size == su * sc`).
        let s = FileLayout {
            stripe_count: 2,
            object_size: 8 << 20,
            ..layout()
        };
        assert_eq!(num_objects(&s, 1), 1);
        assert_eq!(num_objects(&s, 4 << 20), 1);
        assert_eq!(num_objects(&s, 8 << 20), 2);
        assert_eq!(num_objects(&s, (8 << 20) + 1), 3);
    }

    #[test]
    fn truncate_size_follows_striper() {
        let l = layout();
        let m = 4 << 20;
        // Truncate inside object 0.
        assert_eq!(object_truncate_size(&l, 10, 0), Some(10));
        assert_eq!(object_truncate_size(&l, 10, 1), Some(0));
        // Truncate inside object 2: 0 and 1 untouched, 2 at the offset.
        assert_eq!(object_truncate_size(&l, 2 * m + 5, 1), None);
        assert_eq!(object_truncate_size(&l, 2 * m + 5, 2), Some(5));
        assert_eq!(object_truncate_size(&l, 2 * m + 5, 3), Some(0));
    }

    #[test]
    fn overlapping_objects_span_the_range() {
        let l = layout();
        let m = 4 << 20;
        assert_eq!(objects_overlapping(&l, 0, 1), vec![0]);
        assert_eq!(objects_overlapping(&l, m, 1), vec![1]);
        assert_eq!(objects_overlapping(&l, m - 1, 2), vec![0, 1]);
        assert_eq!(objects_overlapping(&l, 3 * m, 2 * m), vec![3, 4]);
    }
}
