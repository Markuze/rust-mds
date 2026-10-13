//! The persistent store of one shard: the encoding of engine journal
//! records (WAL bodies), the WAL segment framing, and (later, per
//! `design/store.md`) the RADOS objects that hold them.
//!
//! The engine stays free of wire dependencies, so its
//! [`engine::JournalRecord`]s are encoded here:
//!
//! - [`record`] gives every record type a versioned `denc` encoding
//!   (v1 today; a newer reader skips unknown tail fields, an older one
//!   refuses a newer `compat`, as everywhere in Ceph).
//! - [`wal`] wraps encoded records in the segment framing of
//!   `design/store.md` section 3: `u32 len | u32 crc32c(body) | u64 seq |
//!   u64 txid | body`. Reading a stream stops at the first record whose
//!   length, crc or sequence number is wrong: that is the torn tail of a
//!   crash, and no prezeroing or probing is needed.

pub mod record;
pub mod wal;
