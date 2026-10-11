//! [`OsdTransport`] over the `msgr` crate: OSD sessions are lossy msgr1 connections opened from
//! the daemon's single messenger (so OSDs see the MDS's own address and entity name, and a
//! blocklist of that address fences every op), authenticated with the `osd` service ticket
//! the `auth` context holds.

use std::sync::Arc;

use denc::{EntityAddrVec, EntityType, RawMessage};
use msgr::{Connection, Event, Messenger};
use tokio::sync::mpsc;
use tracing::debug;

use crate::objecter::{ConnSink, OsdConnection, OsdTransport};

/// Opens OSD sessions on a [`Messenger`].
pub struct MsgrTransport {
    msgr: Messenger,
}

impl MsgrTransport {
    pub fn new(msgr: Messenger) -> Arc<Self> {
        Arc::new(MsgrTransport { msgr })
    }
}

struct MsgrConn(Connection);

impl OsdConnection for MsgrConn {
    fn send(&self, msg: RawMessage) -> bool {
        self.0.send(msg)
    }

    fn features(&self) -> u64 {
        if self.0.is_open() { self.0.features() } else { 0 }
    }

    fn mark_down(&self) {
        self.0.mark_down();
    }
}

impl OsdTransport for MsgrTransport {
    fn connect(&self, osd: i32, addrs: &EntityAddrVec, sink: ConnSink) -> Arc<dyn OsdConnection> {
        // msgr1 only for now: the TYPE_LEGACY entry of the OSD's public address vector
        let addr = addrs.legacy_addr();
        debug!(osd, %addr, "connecting to osd");
        let (tx, mut rx) = mpsc::channel(1024);
        let conn = self.msgr.connect_with(addr, EntityType::OSD, tx);
        tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                match ev {
                    Event::Connected { .. } => sink.connected(),
                    Event::Message { msg, .. } => sink.message(msg),
                    Event::Reset { .. } | Event::Closed { .. } => {
                        sink.reset();
                        break;
                    }
                    Event::Accepted { .. } => {}
                }
            }
        });
        Arc::new(MsgrConn(conn))
    }
}
