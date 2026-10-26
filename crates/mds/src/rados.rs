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

use osdc::objecter::{Objecter, OpError, OpReply, OpRequest};
use osdc::ops::ObjectOperation;
use osdc::types::{ObjectLocator, SnapContext};

use engine::types::FileLayout;

/// The data object `objno` of `ino` (`file_object_t::c_str`: `%llx.%08llx`).
pub fn object_name(ino: u64, objno: u64) -> String {
    format!("{ino:x}.{objno:08x}")
}

/// `Striper::get_num_objects`: how many objects a file of `size` bytes
/// covers under a valid `layout`. Async entry points validate the layout.
pub fn num_objects(layout: &FileLayout, size: u64) -> u64 {
    let su = layout.stripe_unit.max(1) as u64;
    let sc = layout.stripe_count.max(1) as u64;
    let period = layout.object_size.max(1) as u64 * sc;
    let remainder = size % period;
    size / period * sc + remainder.div_ceil(su).min(sc)
}

/// `src/osdc/Striper.cc`, `Striper::object_truncate_size`.
/// `None` means the object is fully before the truncate point.
/// Other objects retain the returned number of object-local bytes.
fn object_truncate_size(layout: &FileLayout, trunc_size: u64, objno: u64) -> Option<u64> {
    let su = layout.stripe_unit.max(1) as u64;
    let sc = layout.stripe_count.max(1) as u64;
    let object_size = layout.object_size.max(1) as u64;
    let period = object_size * sc;
    let objectset = objno / sc;
    let trunc_objectset = trunc_size / period;
    let local = if objectset < trunc_objectset {
        object_size
    } else if objectset > trunc_objectset {
        0
    } else {
        let block = trunc_size % period / su;
        let pos = objno % sc;
        (block / sc) * su
            + if pos < block % sc {
                su
            } else if pos == block % sc {
                trunc_size % su
            } else {
                0
            }
    };
    (local < object_size).then_some(local)
}

/// Object numbers for `[off, off + len)`, once each, in ascending order.
/// Compare object-local prefix lengths (`Striper::file_to_extents`).
/// The iterator uses constant memory, including for large truncate ranges.
fn objects_overlapping(layout: &FileLayout, off: u64, len: u64) -> impl Iterator<Item = u64> + '_ {
    let sc = layout.stripe_count.max(1) as u64;
    let os = layout.object_size.max(1) as u64;
    let end = off.saturating_add(len);
    let first = off / (os * sc) * sc;
    let last = if len == 0 {
        first
    } else {
        num_objects(layout, end)
    };
    (first..last).filter(move |&objno| {
        object_truncate_size(layout, end, objno).unwrap_or(os)
            > object_truncate_size(layout, off, objno).unwrap_or(os)
    })
}

fn validate_layout(layout: &FileLayout) -> Result<(), OpError> {
    if layout.stripe_unit == 0
        || layout.stripe_count == 0
        || layout.object_size < layout.stripe_unit
        || !layout.object_size.is_multiple_of(layout.stripe_unit)
    {
        return Err(OpError::Rados(-22)); // EINVAL
    }
    Ok(())
}

/// Raw `Objecter::operate` leaves OSD errors in both reply result layers.
fn check_reply(reply: OpReply) -> Result<(), OpError> {
    if reply.result < 0 {
        return Err(OpError::Rados(reply.result));
    }
    if let Some(op) = reply.ops.iter().find(|op| op.rval < 0) {
        return Err(OpError::Rados(op.rval));
    }
    Ok(())
}

/// Map the last byte, then add one. Mapping the exclusive object end would
/// skip the other stripe positions at a stripe boundary.
/// See `src/osdc/Striper.cc`, `Striper::get_file_offset`.
fn object_file_end(layout: &FileLayout, objno: u64, size: u64) -> Result<u64, OpError> {
    if size == 0 {
        return Ok(0);
    }
    let su = layout.stripe_unit as u64;
    let sc = layout.stripe_count as u64;
    let os = layout.object_size as u64;
    if size > os {
        return Err(OpError::Rados(-5)); // EIO: object exceeds its layout.
    }
    let last = size - 1;
    (objno / sc)
        .checked_mul(os * sc)
        .and_then(|base| base.checked_add((last / su * sc + objno % sc) * su))
        .and_then(|base| base.checked_add(last % su + 1))
        .ok_or(OpError::Rados(-5))
}

/// Fold one STAT result. Missing objects do not contribute size or mtime.
/// Empty existing objects contribute mtime, but never a false file offset.
fn record_probe(
    layout: &FileLayout,
    objno: u64,
    stat: Result<(u64, denc::UTime), OpError>,
    recovered: &mut Option<(u64, denc::UTime)>,
) -> Result<(), OpError> {
    let (size, mtime) = match stat {
        Ok(stat) => stat,
        Err(OpError::Rados(-2)) => return Ok(()),
        Err(e) => return Err(e),
    };
    let end = object_file_end(layout, objno, size)?;
    let current = recovered.get_or_insert((0, denc::UTime::default()));
    current.0 = current.0.max(end);
    if (mtime.sec, mtime.nsec) > (current.1.sec, current.1.nsec) {
        current.1 = mtime;
    }
    Ok(())
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
/// the layout pool and namespace. Ignore `-ENOENT`; return other failures.
/// A caller may retry the complete range after partial progress.
pub async fn purge_inode(
    objecter: &Objecter,
    ino: u64,
    layout: &FileLayout,
    size: u64,
) -> Result<(), OpError> {
    validate_layout(layout)?;
    let oloc = oloc_for(layout);
    for objno in 0..num_objects(layout, size) {
        let oid = object_name(ino, objno);
        match objecter
            .remove(oloc.clone(), oid.clone(), snapc_head())
            .await
            .and_then(check_reply)
        {
            Ok(()) => {}
            Err(OpError::Rados(-2)) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
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
) -> Result<(), OpError> {
    validate_layout(layout)?;
    if from <= size {
        return Ok(());
    }
    let oloc = oloc_for(layout);
    for objno in objects_overlapping(layout, size, from - size) {
        let Some(local) = object_truncate_size(layout, size, objno) else {
            continue;
        };
        let mut op = ObjectOperation::new();
        op.trimtrunc(truncate_seq, local);
        let reply = objecter
            .operate(OpRequest::new(oloc.clone(), object_name(ino, objno), op))
            .await?;
        check_reply(reply)?;
    }
    Ok(())
}

/// The recovered size and mtime of a file whose writer was lost
/// (`RecoverFileSize`): probe objects backwards from `max_size`
/// (`Filer::probe`). Return the largest mapped end and maximum mtime across
/// all candidate objects. `Ok(None)` means no objects, not a failed probe.
/// A zero bound has no candidates. The caller must fence lost writers first.
pub async fn recover_file_size(
    objecter: &Objecter,
    ino: u64,
    layout: &FileLayout,
    max_size: u64,
) -> Result<Option<(u64, denc::UTime)>, OpError> {
    validate_layout(layout)?;
    let oloc = oloc_for(layout);
    let mut recovered = None;
    for objno in (0..num_objects(layout, max_size)).rev() {
        record_probe(
            layout,
            objno,
            objecter.stat(oloc.clone(), object_name(ino, objno)).await,
            &mut recovered,
        )?;
    }
    if recovered.as_ref().is_some_and(|(size, _)| *size > max_size) {
        return Err(OpError::Rados(-5));
    }
    Ok(recovered)
}

/// Execute a committed durable intent. Errors retain the intent in the engine;
/// only the daemon's later completion transaction may retire it.
pub async fn execute_lifecycle(
    objecter: &Objecter,
    intent: &engine::LifecycleIntent,
) -> Result<engine::LifecycleResult, OpError> {
    use engine::{LifecycleKind, LifecycleResult, Timestamp};
    lifecycle_budget(intent)?;
    match &intent.kind {
        LifecycleKind::Purge { layouts, max_size } => {
            for layout in layouts {
                if !layout.is_set() {
                    continue;
                } // Metadata-only directory intent.
                purge_inode(objecter, intent.ino, layout, *max_size).await?;
            }
            Ok(LifecycleResult::Done)
        }
        LifecycleKind::Truncate {
            layout,
            from,
            size,
            truncate_seq,
        } => {
            truncate_data(objecter, intent.ino, layout, *truncate_seq, *size, *from).await?;
            Ok(LifecycleResult::Done)
        }
        LifecycleKind::RecoverSize {
            layouts, max_size, ..
        } => {
            if layouts.is_empty() || layouts.iter().any(|layout| !layout.is_set()) {
                return Err(OpError::Rados(-22));
            }
            let mut size = 0;
            let mut mtime = denc::UTime::default();
            for layout in layouts {
                if let Some((end, time)) =
                    recover_file_size(objecter, intent.ino, layout, *max_size).await?
                {
                    size = size.max(end);
                    if (time.sec, time.nsec) > (mtime.sec, mtime.nsec) {
                        mtime = time;
                    }
                }
            }
            Ok(LifecycleResult::Recovered {
                size,
                mtime: Timestamp::new(mtime.sec, mtime.nsec),
            })
        }
    }
}

/// Bound aggregate object operations before the first destructive operation.
/// Large jobs stay durable and report E2BIG instead of partially looping forever.
fn lifecycle_budget(intent: &engine::LifecycleIntent) -> Result<(), OpError> {
    const MAX_OBJECTS: u64 = 4096;
    const MAX_LAYOUTS: usize = 64;
    let mut total = 0u64;
    let mut add = |layout: &FileLayout, size: u64| {
        validate_layout(layout)?;
        total = total
            .checked_add(num_objects(layout, size))
            .ok_or(OpError::Rados(-7))?;
        if total > MAX_OBJECTS {
            return Err(OpError::Rados(-7));
        }
        Ok(())
    };
    match &intent.kind {
        engine::LifecycleKind::Purge { layouts, max_size } => {
            if layouts.len() > MAX_LAYOUTS {
                return Err(OpError::Rados(-7));
            }
            for layout in layouts {
                if layout.is_set() {
                    add(layout, *max_size)?;
                }
            }
        }
        engine::LifecycleKind::RecoverSize {
            layouts, max_size, ..
        } => {
            if layouts.is_empty() || layouts.len() > MAX_LAYOUTS {
                return Err(OpError::Rados(-7));
            }
            for layout in layouts {
                add(layout, *max_size)?;
            }
        }
        engine::LifecycleKind::Truncate { layout, from, .. } => add(layout, *from)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn lifecycle_preflight_bounds_aggregate_objects_before_execution() {
        let layout = layout();
        let mut intent = engine::LifecycleIntent {
            id: 1,
            ino: 123,
            kind: engine::LifecycleKind::Purge {
                layouts: vec![layout.clone()],
                max_size: layout.object_size as u64 * 4096,
            },
        };
        assert!(super::lifecycle_budget(&intent).is_ok());
        intent.kind = engine::LifecycleKind::Purge {
            layouts: vec![layout.clone(), layout.clone()],
            max_size: layout.object_size as u64 * 4096,
        };
        assert!(super::lifecycle_budget(&intent).is_err());
        intent.kind = engine::LifecycleKind::RecoverSize {
            layouts: vec![layout],
            max_size: u64::MAX,
            inode_version: 1,
            truncate_seq: 0,
        };
        assert!(super::lifecycle_budget(&intent).is_err());
    }
    use super::*;

    fn objects_overlapping(layout: &FileLayout, off: u64, len: u64) -> Vec<u64> {
        super::objects_overlapping(layout, off, len).collect()
    }

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
        // Two stripes per object. A period contains SC complete objects.
        let s = FileLayout {
            stripe_count: 2,
            object_size: 8 << 20,
            ..layout()
        };
        assert_eq!(num_objects(&s, 1), 1);
        assert_eq!(num_objects(&s, 4 << 20), 1);
        assert_eq!(num_objects(&s, 8 << 20), 2);
        assert_eq!(num_objects(&s, (8 << 20) + 1), 2);
        assert_eq!(num_objects(&s, 16 << 20), 2);
        assert_eq!(num_objects(&s, (16 << 20) + 1), 3);
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

    #[test]
    fn striped_truncate_reference_vector() {
        // Striper::object_truncate_size: SU=4, SC=3, OS=8.
        let l = FileLayout {
            stripe_unit: 4,
            stripe_count: 3,
            object_size: 8,
            ..layout()
        };
        assert_eq!(object_truncate_size(&l, 14, 0), Some(6));
        assert_eq!(object_truncate_size(&l, 14, 1), Some(4));
        assert_eq!(object_truncate_size(&l, 14, 2), Some(4));
        assert_eq!(object_truncate_size(&l, 14, 3), Some(0));
    }

    #[test]
    fn striped_overlap_reference_vector() {
        let l = FileLayout {
            stripe_unit: 4,
            stripe_count: 3,
            object_size: 8,
            ..layout()
        };
        assert_eq!(objects_overlapping(&l, 4, 1), vec![1]);
        assert_eq!(objects_overlapping(&l, 14, 4), vec![0, 1]);
        assert_eq!(objects_overlapping(&l, 0, 0), Vec::<u64>::new());
    }

    #[test]
    fn file_end_reference_vectors() {
        // SU=4, SC=3, OS=8. Object bytes 0..4 map to file offsets
        // 0..4, 4..8, 8..12; bytes 4..8 map to 12..16, 16..20, 20..24.
        let l = FileLayout {
            stripe_unit: 4,
            stripe_count: 3,
            object_size: 8,
            ..layout()
        };
        for (objno, size, end) in [
            (0, 0, 0),
            (2, 0, 0),
            (0, 4, 4),
            (1, 4, 8),
            (2, 4, 12),
            (0, 5, 13),
            (0, 8, 16),
            (1, 8, 20),
            (2, 8, 24),
            (3, 1, 25),
        ] {
            assert_eq!(object_file_end(&l, objno, size), Ok(end));
        }
        assert_eq!(object_file_end(&l, 0, 9), Err(OpError::Rados(-5)));
    }

    #[test]
    fn sparse_probe_uses_maximum_end_and_all_mtimes() {
        let l = FileLayout {
            stripe_unit: 4,
            stripe_count: 3,
            object_size: 8,
            ..layout()
        };
        let mut recovered = None;
        record_probe(&l, 5, Err(OpError::Rados(-2)), &mut recovered).unwrap();
        assert_eq!(recovered, None);
        // Empty high-numbered objects must not extend file size.
        record_probe(
            &l,
            4,
            Ok((0, denc::UTime { sec: 8, nsec: 1 })),
            &mut recovered,
        )
        .unwrap();
        assert_eq!(recovered, Some((0, denc::UTime { sec: 8, nsec: 1 })));
        record_probe(
            &l,
            2,
            Ok((1, denc::UTime { sec: 1, nsec: 0 })),
            &mut recovered,
        )
        .unwrap();
        // A lower-numbered object can contain the final file byte.
        record_probe(
            &l,
            0,
            Ok((8, denc::UTime { sec: 2, nsec: 0 })),
            &mut recovered,
        )
        .unwrap();
        assert_eq!(recovered, Some((16, denc::UTime { sec: 8, nsec: 1 })));
        for error in [
            OpError::TimedOut,
            OpError::PoolDoesNotExist,
            OpError::Blocklisted,
            OpError::Rados(-5),
        ] {
            assert_eq!(
                record_probe(&l, 0, Err(error.clone()), &mut recovered),
                Err(error)
            );
        }
    }

    #[test]
    fn truncate_rejects_both_osd_error_layers() {
        use osdc::objecter::OpResult;
        assert_eq!(check_reply(OpReply::default()), Ok(()));
        assert_eq!(
            check_reply(OpReply {
                result: -5,
                ..OpReply::default()
            }),
            Err(OpError::Rados(-5))
        );
        assert_eq!(
            check_reply(OpReply {
                ops: vec![OpResult {
                    rval: -22,
                    ..OpResult::default()
                }],
                ..OpReply::default()
            }),
            Err(OpError::Rados(-22))
        );
    }

    #[test]
    fn small_layouts_match_independent_byte_mapping() {
        for (su, sc, os) in [(1, 1, 1), (2, 1, 8), (2, 3, 8), (4, 2, 8)] {
            let l = FileLayout {
                stripe_unit: su,
                stripe_count: sc,
                object_size: os,
                ..layout()
            };
            let period = (sc * os) as u64;
            // Forward byte mapping from Striper::file_to_extents.
            let object_for = |offset: u64| {
                let block = offset / su as u64;
                block / sc as u64 / (os / su) as u64 * sc as u64 + block % sc as u64
            };
            for end in 0..=2 * period + 1 {
                let expected_count = (0..end).map(object_for).max().map_or(0, |n| n + 1);
                assert_eq!(num_objects(&l, end), expected_count);
                for objno in 0..3 * sc as u64 {
                    let local = (0..end)
                        .filter(|&offset| object_for(offset) == objno)
                        .count() as u64;
                    assert_eq!(
                        object_truncate_size(&l, end, objno).unwrap_or(os as u64),
                        local
                    );
                }
                for off in 0..=end {
                    let mut expected: Vec<_> = (off..end).map(object_for).collect();
                    expected.sort_unstable();
                    expected.dedup();
                    assert_eq!(objects_overlapping(&l, off, end - off), expected);
                }
            }
        }
    }

    #[test]
    fn large_ranges_do_not_allocate_object_vectors() {
        let l = FileLayout {
            stripe_unit: 1,
            stripe_count: 2,
            object_size: 2,
            ..layout()
        };
        assert_eq!(
            super::objects_overlapping(&l, 0, u64::MAX)
                .take(3)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        let l = FileLayout {
            stripe_unit: 1,
            stripe_count: 2,
            object_size: 1,
            ..layout()
        };
        assert_eq!(num_objects(&l, u64::MAX), u64::MAX);
    }

    #[test]
    fn rejects_invalid_layouts() {
        for l in [
            FileLayout {
                stripe_unit: 0,
                ..layout()
            },
            FileLayout {
                stripe_count: 0,
                ..layout()
            },
            FileLayout {
                object_size: 0,
                ..layout()
            },
            FileLayout {
                stripe_unit: 3,
                object_size: 8,
                ..layout()
            },
        ] {
            assert_eq!(validate_layout(&l), Err(OpError::Rados(-22)));
        }
    }

    #[test]
    fn preserves_job_pool_and_namespace() {
        // A retained old layout must not follow a later default-pool change.
        let old = FileLayout {
            pool_id: 71,
            pool_ns: "retired-data".into(),
            ..layout()
        };
        let oloc = oloc_for(&old);
        assert_eq!(oloc.pool, 71);
        assert_eq!(oloc.nspace, Bytes::from_static(b"retired-data"));
    }
}
