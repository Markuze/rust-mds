//! OSD client for the MDS's own RADOS I/O, natively in Rust.
//!
//! - [`osdmap`]: `OSDMap` and `OSDMap::Incremental` decoding (as monitors and OSDs send them in
//!   `MOSDMap`), applying incrementals, blocklist checks, and the placement chain
//!   (object → raw pg → pg → CRUSH → upmaps → up set → primary affinity → pg_temp/primary_temp
//!   → acting set and primary).
//! - [`crush`]: CRUSH map decoding and `crush_do_rule` (uniform/list/tree/straw/straw2 buckets,
//!   firstn/indep, MSR rules, tunables, choose_args), bit-exact with `src/crush/mapper.c`.
//! - [`pool`]: `pg_pool_t`; [`hash`]: `ceph_str_hash` and the CRUSH hashes; [`types`]: `pg_t`,
//!   `spg_t`, `object_locator_t`, `hobject_t`, `osd_reqid_t`, ...
//! - [`messages`]: `MOSDMap`, `MOSDOp` (v9/v8), `MOSDOpReply`, `MOSDBackoff`.
//! - [`ops`]: op codes and the [`ObjectOperation`] builder (plain RADOS ops only: the MDS's
//!   cephx OSD caps have no `x`).
//! - [`objecter`]: the async client (targets, resends, pauses, epoch barrier, backoffs,
//!   `-EAGAIN`/redirects, `-EBLOCKLISTED`, `FULL_FORCE` on every op) over the
//!   [`objecter::OsdTransport`] trait; [`msgr_transport`] binds it to the `msgr` crate so that
//!   all OSD traffic leaves from the daemon's single messenger identity.
//!
//! Helpers that `denc` lacks (varints, `float`, legacy-compat frames, the raw Ceph CRC32C) are
//! implemented locally in `util`.
//!
//! # Wiring (see `examples/rados_live.rs`)
//!
//! ```no_run
//! # async fn wire(msgr: msgr::Messenger, inc: u32) {
//! use osdc::{Objecter, ObjecterConfig, ObjecterEvent};
//! let cfg = ObjecterConfig { client_inc: inc, ..Default::default() }; // MDSMap incarnation
//! let (objecter, mut events) = Objecter::start(osdc::msgr_transport::MsgrTransport::new(msgr), cfg);
//! // MonClient MOSDMap (type 41) messages -> objecter.handle_osd_map(MOsdMap::from_raw(raw)?)
//! while let Some(ev) = events.recv().await {
//!     match ev {
//!         // monc.subscribe("osdmap", start, if onetime { SUBSCRIBE_ONETIME } else { 0 })
//!         ObjecterEvent::RequestMap { start, onetime } => {}
//!         // monc get_version("osdmap") -> objecter.set_latest_map_version(v)
//!         ObjecterEvent::CheckLatestMap => {}
//!         // monc.sub_got("osdmap", epoch); MDS per-map work (is_full, blocklist scan, ...)
//!         ObjecterEvent::MapApplied { epoch } => {}
//!         // client sessions to evict (OSDMap blocklist)
//!         ObjecterEvent::Blocklisted { addrs } => {}
//!         // -EBLOCKLISTED: this daemon is fenced and must respawn
//!         ObjecterEvent::Fenced => {}
//!     }
//! }
//! # }
//! ```

pub mod crush;
pub mod hash;
pub mod messages;
pub mod msgr_transport;
pub mod objecter;
pub mod ops;
pub mod osdmap;
pub mod pool;
pub mod types;
mod util;

pub use crush::CrushMap;
pub use messages::{MOsdBackoff, MOsdMap, MOsdOp, MOsdOpReply};
pub use objecter::{Objecter, ObjecterConfig, ObjecterEvent, OpError, OpReply, OpRequest};
pub use ops::{ObjectOperation, OsdOp};
pub use osdmap::{Incremental, ObjectPlacement, OsdMap, UpActing};
pub use pool::PgPool;
pub use types::{HObject, ObjectLocator, OsdReqId, PgId, SnapContext, SpgId, file_object_name};
pub use util::ceph_crc32c;
