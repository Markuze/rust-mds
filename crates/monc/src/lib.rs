//! MonClient: monitor sessions, cephx with the monitors, subscriptions, MDSMap/FSMap decoding,
//! beacons.
//!
//! - [`MonClient`]: hunting, msgr1 `MAuth` cephx handshake, ticket renewal, rotating secrets,
//!   subscriptions, keepalives ([`monclient`]).
//! - [`MdsMap`], [`MdsInfo`], [`CompatSet`], [`MdsState`] ([`mdsmap`]).
//! - [`Beacon`]: periodic `MMDSBeacon` with ack tracking ([`beacon`]).
//! - Monitor messages ([`messages`]), the MonMap ([`monmap`]) and `ceph.conf` reading
//!   ([`config`]).

pub mod beacon;
pub mod config;
pub mod mdsmap;
pub mod messages;
pub mod monclient;
pub mod monmap;

pub use beacon::{AddrsString, Beacon, BeaconConfig, BeaconStats};
pub use config::{CephConf, parse_mon_host_v1};
pub use mdsmap::{CompatSet, FeatureSet, MdsInfo, MdsMap, MdsState};
pub use messages::{MMDSBeacon, MMDSMap, MdsHealth, MdsHealthMetric};
pub use monclient::{MonClient, MonClientConfig, MonEvent, MonStatus};
pub use monmap::{MonInfo, MonMap};

use denc::{Message, RawMessage};

/// Decode an `MMDSMap` message and the map it carries.
pub fn decode_mdsmap(raw: RawMessage) -> Result<(MMDSMap, MdsMap), denc::DecodeError> {
    let m = MMDSMap::from_raw(raw)?;
    let mut b = m.encoded.clone();
    let map = <MdsMap as denc::Decode>::decode(&mut b)?;
    Ok((m, map))
}
