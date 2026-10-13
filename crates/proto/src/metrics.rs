//! `MClientMetrics` (`CEPH_MSG_CLIENT_METRICS`, `src/messages/MClientMetrics.h`), version 1
//! compat 1, priority `CEPH_MSG_PRIO_HIGH - 1`, client to MDS.
//!
//! Front: `u32 count`, then count items of `u32 metric_type` + an `ENCODE_START` frame
//! (`ClientMetricMessage`, `src/include/cephfs/metrics/Types.h`). The decoder is tolerant: an
//! item of an unknown type, of an incompatible struct version, or whose payload does not parse
//! is kept as [`ClientMetric::Unknown`] and skipped by its frame length; only a malformed frame
//! (truncated header or body) fails the message.

use std::fmt;

use denc::{Buf, BufMut, Bytes, BytesMut, Encode, Message, Payload, Result, UTime};

use crate::msg_type;
use crate::util::{Ctx, get};

/// `ClientMetricType` values.
pub mod metric_type {
    pub const CAP_INFO: u32 = 0;
    pub const READ_LATENCY: u32 = 1;
    pub const WRITE_LATENCY: u32 = 2;
    pub const METADATA_LATENCY: u32 = 3;
    pub const DENTRY_LEASE: u32 = 4;
    pub const OPENED_FILES: u32 = 5;
    pub const PINNED_ICAPS: u32 = 6;
    pub const OPENED_INODES: u32 = 7;
    pub const READ_IO_SIZES: u32 = 8;
    pub const WRITE_IO_SIZES: u32 = 9;
    pub const AVG_READ_LATENCY: u32 = 10;
    pub const STDEV_READ_LATENCY: u32 = 11;
    pub const AVG_WRITE_LATENCY: u32 = 12;
    pub const STDEV_WRITE_LATENCY: u32 = 13;
    pub const AVG_METADATA_LATENCY: u32 = 14;
    pub const STDEV_METADATA_LATENCY: u32 = 15;
    pub const SUBVOLUME_METRICS: u32 = 16;
}

/// Latency payload (`ReadLatencyPayload` etc.), `ENCODE_START(2, 1)` { `utime lat` (sum),
/// v2: `utime mean`, `u64 sq_sum` (ns^2), `u64 count` }.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LatencyMetric {
    pub lat: UTime,
    pub mean: UTime,
    pub sq_sum: u64,
    pub count: u64,
}

/// `AggregatedIOMetrics`: per-subvolume I/O since the last report, `ENCODE_START(1, 1)` {
/// `u64 subvolume_id`, `u32 read_count`, `u32 write_count`, `u64 read_bytes`, `u64 write_bytes`,
/// `u64 read_latency_us`, `u64 write_latency_us`, `u64 time_stamp` }.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SubvolumeIoMetric {
    pub subvolume_id: u64,
    pub read_count: u32,
    pub write_count: u32,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_latency_us: u64,
    pub write_latency_us: u64,
    pub time_stamp: u64,
}

/// One metric item.
#[derive(Clone, PartialEq, Eq)]
pub enum ClientMetric {
    CapInfo {
        hits: u64,
        misses: u64,
        total_caps: u64,
    },
    ReadLatency(LatencyMetric),
    WriteLatency(LatencyMetric),
    MetadataLatency(LatencyMetric),
    DentryLease {
        hits: u64,
        misses: u64,
        total_dentries: u64,
    },
    OpenedFiles {
        opened_files: u64,
        total_inodes: u64,
    },
    PinnedIcaps {
        pinned_icaps: u64,
        total_inodes: u64,
    },
    OpenedInodes {
        opened_inodes: u64,
        total_inodes: u64,
    },
    ReadIoSizes {
        total_ops: u64,
        total_size: u64,
    },
    WriteIoSizes {
        total_ops: u64,
        total_size: u64,
    },
    SubvolumeMetrics(Vec<SubvolumeIoMetric>),
    /// Any other item, kept with its complete frame (header included).
    Unknown {
        metric_type: u32,
        frame: Bytes,
    },
}

impl fmt::Debug for ClientMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientMetric::Unknown { metric_type, frame } => {
                write!(f, "Unknown(type {metric_type}, {} bytes)", frame.len())
            }
            ClientMetric::CapInfo {
                hits,
                misses,
                total_caps,
            } => write!(f, "CapInfo({hits}/{misses}/{total_caps})"),
            ClientMetric::ReadLatency(l) => write!(f, "ReadLatency({l:?})"),
            ClientMetric::WriteLatency(l) => write!(f, "WriteLatency({l:?})"),
            ClientMetric::MetadataLatency(l) => write!(f, "MetadataLatency({l:?})"),
            ClientMetric::DentryLease {
                hits,
                misses,
                total_dentries,
            } => {
                write!(f, "DentryLease({hits}/{misses}/{total_dentries})")
            }
            ClientMetric::OpenedFiles {
                opened_files,
                total_inodes,
            } => {
                write!(f, "OpenedFiles({opened_files}/{total_inodes})")
            }
            ClientMetric::PinnedIcaps {
                pinned_icaps,
                total_inodes,
            } => {
                write!(f, "PinnedIcaps({pinned_icaps}/{total_inodes})")
            }
            ClientMetric::OpenedInodes {
                opened_inodes,
                total_inodes,
            } => {
                write!(f, "OpenedInodes({opened_inodes}/{total_inodes})")
            }
            ClientMetric::ReadIoSizes {
                total_ops,
                total_size,
            } => write!(f, "ReadIoSizes({total_ops}/{total_size})"),
            ClientMetric::WriteIoSizes {
                total_ops,
                total_size,
            } => write!(f, "WriteIoSizes({total_ops}/{total_size})"),
            ClientMetric::SubvolumeMetrics(v) => write!(f, "SubvolumeMetrics({v:?})"),
        }
    }
}

impl ClientMetric {
    pub fn metric_type(&self) -> u32 {
        use metric_type::*;
        match self {
            ClientMetric::CapInfo { .. } => CAP_INFO,
            ClientMetric::ReadLatency(_) => READ_LATENCY,
            ClientMetric::WriteLatency(_) => WRITE_LATENCY,
            ClientMetric::MetadataLatency(_) => METADATA_LATENCY,
            ClientMetric::DentryLease { .. } => DENTRY_LEASE,
            ClientMetric::OpenedFiles { .. } => OPENED_FILES,
            ClientMetric::PinnedIcaps { .. } => PINNED_ICAPS,
            ClientMetric::OpenedInodes { .. } => OPENED_INODES,
            ClientMetric::ReadIoSizes { .. } => READ_IO_SIZES,
            ClientMetric::WriteIoSizes { .. } => WRITE_IO_SIZES,
            ClientMetric::SubvolumeMetrics(_) => SUBVOLUME_METRICS,
            ClientMetric::Unknown { metric_type, .. } => *metric_type,
        }
    }

    fn encode_item(&self, b: &mut BytesMut) {
        b.put_u32_le(self.metric_type());
        let u64s = |b: &mut BytesMut, v: &[u64]| {
            denc::encode_versioned(b, 1, 1, |b| v.iter().for_each(|x| b.put_u64_le(*x)));
        };
        let lat = |b: &mut BytesMut, l: &LatencyMetric| {
            denc::encode_versioned(b, 2, 1, |b| {
                l.lat.encode(b, 0);
                l.mean.encode(b, 0);
                b.put_u64_le(l.sq_sum);
                b.put_u64_le(l.count);
            });
        };
        match self {
            ClientMetric::CapInfo {
                hits,
                misses,
                total_caps,
            } => u64s(b, &[*hits, *misses, *total_caps]),
            ClientMetric::ReadLatency(l)
            | ClientMetric::WriteLatency(l)
            | ClientMetric::MetadataLatency(l) => lat(b, l),
            ClientMetric::DentryLease {
                hits,
                misses,
                total_dentries,
            } => u64s(b, &[*hits, *misses, *total_dentries]),
            ClientMetric::OpenedFiles {
                opened_files: a,
                total_inodes: t,
            }
            | ClientMetric::PinnedIcaps {
                pinned_icaps: a,
                total_inodes: t,
            }
            | ClientMetric::OpenedInodes {
                opened_inodes: a,
                total_inodes: t,
            } => u64s(b, &[*a, *t]),
            ClientMetric::ReadIoSizes {
                total_ops,
                total_size,
            }
            | ClientMetric::WriteIoSizes {
                total_ops,
                total_size,
            } => u64s(b, &[*total_ops, *total_size]),
            ClientMetric::SubvolumeMetrics(v) => denc::encode_versioned(b, 1, 1, |b| {
                b.put_u64_le(v.len() as u64);
                for m in v {
                    denc::encode_versioned(b, 1, 1, |b| {
                        b.put_u64_le(m.subvolume_id);
                        b.put_u32_le(m.read_count);
                        b.put_u32_le(m.write_count);
                        b.put_u64_le(m.read_bytes);
                        b.put_u64_le(m.write_bytes);
                        b.put_u64_le(m.read_latency_us);
                        b.put_u64_le(m.write_latency_us);
                        b.put_u64_le(m.time_stamp);
                    });
                }
            }),
            ClientMetric::Unknown { frame, .. } => b.put_slice(frame),
        }
    }

    /// Parse a known payload from its frame; `None` if unknown, incompatible or malformed.
    fn parse_known(metric_type: u32, frame: &Bytes) -> Option<ClientMetric> {
        use metric_type::*;
        let mut buf = frame.clone();
        let supported = match metric_type {
            READ_LATENCY | WRITE_LATENCY | METADATA_LATENCY => 2,
            CAP_INFO | DENTRY_LEASE | OPENED_FILES | PINNED_ICAPS | OPENED_INODES
            | READ_IO_SIZES | WRITE_IO_SIZES | SUBVOLUME_METRICS => 1,
            _ => return None,
        };
        let mut f = denc::decode_start(&mut buf, supported, "metric").ok()?;
        let b = &mut f.body;
        let u = |b: &mut Bytes| get::<u64>(b, "metric");
        let lat = |b: &mut Bytes, v: u8| -> Result<LatencyMetric> {
            let lat = get::<UTime>(b, "latency")?;
            if v >= 2 {
                Ok(LatencyMetric {
                    lat,
                    mean: get(b, "mean")?,
                    sq_sum: get(b, "sq_sum")?,
                    count: get(b, "count")?,
                })
            } else {
                Ok(LatencyMetric {
                    lat,
                    ..Default::default()
                })
            }
        };
        let r: Result<ClientMetric> = (|| {
            Ok(match metric_type {
                CAP_INFO => ClientMetric::CapInfo {
                    hits: u(b)?,
                    misses: u(b)?,
                    total_caps: u(b)?,
                },
                READ_LATENCY => ClientMetric::ReadLatency(lat(b, f.v)?),
                WRITE_LATENCY => ClientMetric::WriteLatency(lat(b, f.v)?),
                METADATA_LATENCY => ClientMetric::MetadataLatency(lat(b, f.v)?),
                DENTRY_LEASE => ClientMetric::DentryLease {
                    hits: u(b)?,
                    misses: u(b)?,
                    total_dentries: u(b)?,
                },
                OPENED_FILES => ClientMetric::OpenedFiles {
                    opened_files: u(b)?,
                    total_inodes: u(b)?,
                },
                PINNED_ICAPS => ClientMetric::PinnedIcaps {
                    pinned_icaps: u(b)?,
                    total_inodes: u(b)?,
                },
                OPENED_INODES => ClientMetric::OpenedInodes {
                    opened_inodes: u(b)?,
                    total_inodes: u(b)?,
                },
                READ_IO_SIZES => ClientMetric::ReadIoSizes {
                    total_ops: u(b)?,
                    total_size: u(b)?,
                },
                WRITE_IO_SIZES => ClientMetric::WriteIoSizes {
                    total_ops: u(b)?,
                    total_size: u(b)?,
                },
                SUBVOLUME_METRICS => {
                    let n = u(b)?;
                    let mut v = Vec::new();
                    for _ in 0..n {
                        let mut e = denc::decode_start(b, 1, "AggregatedIOMetrics")?;
                        let e = &mut e.body;
                        v.push(SubvolumeIoMetric {
                            subvolume_id: get(e, "subvolume_id")?,
                            read_count: get(e, "read_count")?,
                            write_count: get(e, "write_count")?,
                            read_bytes: get(e, "read_bytes")?,
                            write_bytes: get(e, "write_bytes")?,
                            read_latency_us: get(e, "read_latency_us")?,
                            write_latency_us: get(e, "write_latency_us")?,
                            time_stamp: get(e, "time_stamp")?,
                        });
                    }
                    ClientMetric::SubvolumeMetrics(v)
                }
                _ => unreachable!("filtered above"),
            })
        })();
        r.ok()
    }
}

/// `MClientMetrics`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MClientMetrics {
    pub metrics: Vec<ClientMetric>,
}

impl Message for MClientMetrics {
    const TYPE: u16 = msg_type::CLIENT_METRICS;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 1;
    const PRIORITY: u16 = denc::msg::PRIO_HIGH - 1;

    fn encode_payload(&self, _features: u64) -> Payload {
        let mut b = BytesMut::new();
        b.put_u32_le(self.metrics.len() as u32);
        for m in &self.metrics {
            m.encode_item(&mut b);
        }
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(
        _version: u16,
        mut front: Bytes,
        _middle: Bytes,
        _data: Bytes,
    ) -> Result<Self> {
        let p = &mut front;
        let n = get::<u32>(p, "MClientMetrics.count")?;
        let mut metrics = Vec::with_capacity((n as usize).min(p.remaining() / 10));
        for _ in 0..n {
            let metric_type = get::<u32>(p, "MClientMetrics.type")?;
            denc::need(p, 6, "MClientMetrics item frame")?;
            let len = u32::from_le_bytes(p[2..6].try_into().unwrap()) as usize;
            let frame = denc::take(p, 6usize.saturating_add(len), "MClientMetrics item")
                .ctx("MClientMetrics item")?;
            let m = ClientMetric::parse_known(metric_type, &frame)
                .unwrap_or(ClientMetric::Unknown { metric_type, frame });
            metrics.push(m);
        }
        Ok(MClientMetrics { metrics })
    }

    fn from_raw(raw: denc::RawMessage) -> Result<Self> {
        crate::util::check_raw::<Self>(&raw)?;
        Self::decode_payload(raw.version, raw.front, raw.middle, raw.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel_like() -> MClientMetrics {
        MClientMetrics {
            metrics: vec![
                ClientMetric::CapInfo {
                    hits: 10,
                    misses: 2,
                    total_caps: 5,
                },
                ClientMetric::ReadLatency(LatencyMetric {
                    lat: UTime { sec: 1, nsec: 2 },
                    mean: UTime { sec: 0, nsec: 3 },
                    sq_sum: 4,
                    count: 5,
                }),
                ClientMetric::WriteLatency(LatencyMetric::default()),
                ClientMetric::MetadataLatency(LatencyMetric::default()),
                ClientMetric::DentryLease {
                    hits: 1,
                    misses: 1,
                    total_dentries: 1,
                },
                ClientMetric::OpenedFiles {
                    opened_files: 0,
                    total_inodes: 3,
                },
                ClientMetric::PinnedIcaps {
                    pinned_icaps: 3,
                    total_inodes: 3,
                },
                ClientMetric::OpenedInodes {
                    opened_inodes: 0,
                    total_inodes: 3,
                },
                ClientMetric::ReadIoSizes {
                    total_ops: 0,
                    total_size: 0,
                },
                ClientMetric::WriteIoSizes {
                    total_ops: 1,
                    total_size: 4096,
                },
                ClientMetric::SubvolumeMetrics(vec![SubvolumeIoMetric {
                    subvolume_id: 0x1000,
                    read_count: 1,
                    write_bytes: 9,
                    ..Default::default()
                }]),
            ],
        }
    }

    #[test]
    fn round_trip_and_item_bytes() {
        let m = kernel_like();
        let raw = m.to_raw(0);
        assert_eq!((raw.version, raw.compat_version, raw.priority), (1, 1, 195));
        // First item: type 0, ENCODE_START(1,1) len 24, three u64.
        assert_eq!(
            hex::encode(&raw.front[..14]),
            "0b000000".to_owned() + "00000000" + "010118000000"
        );
        assert_eq!(MClientMetrics::from_raw(raw).unwrap(), m);
    }

    #[test]
    fn unknown_and_broken_items_are_skipped() {
        let mut b = BytesMut::new();
        b.put_u32_le(4);
        // An unknown type (99) with a future struct version.
        b.put_u32_le(99);
        denc::encode_versioned(&mut b, 7, 5, |b| b.put_slice(b"whatever"));
        // A known type with an incompatible compat version.
        b.put_u32_le(metric_type::CAP_INFO);
        denc::encode_versioned(&mut b, 3, 3, |b| b.put_bytes(0, 24));
        // A known type whose body is too short.
        b.put_u32_le(metric_type::OPENED_FILES);
        denc::encode_versioned(&mut b, 1, 1, |b| b.put_u64_le(1));
        // A known type with extra trailing body bytes (newer client): still parsed.
        b.put_u32_le(metric_type::READ_IO_SIZES);
        denc::encode_versioned(&mut b, 2, 1, |b| {
            b.put_u64_le(7);
            b.put_u64_le(8);
            b.put_u32_le(0xffff);
        });
        let m = MClientMetrics::decode_payload(1, b.freeze(), Bytes::new(), Bytes::new()).unwrap();
        assert_eq!(m.metrics.len(), 4);
        assert!(matches!(
            m.metrics[0],
            ClientMetric::Unknown {
                metric_type: 99,
                ..
            }
        ));
        assert!(matches!(
            m.metrics[1],
            ClientMetric::Unknown { metric_type: 0, .. }
        ));
        assert!(matches!(
            m.metrics[2],
            ClientMetric::Unknown { metric_type: 5, .. }
        ));
        assert_eq!(
            m.metrics[3],
            ClientMetric::ReadIoSizes {
                total_ops: 7,
                total_size: 8
            }
        );
        // Unknown items re-encode byte for byte.
        let again = MClientMetrics::decode_payload(
            1,
            m.encode_payload(0).front.freeze(),
            Bytes::new(),
            Bytes::new(),
        )
        .unwrap();
        assert_eq!(again.metrics[..3], m.metrics[..3]);
        // A truncated frame fails the message.
        let mut b = BytesMut::new();
        b.put_u32_le(1);
        b.put_u32_le(99);
        b.put_slice(&[1, 1, 10, 0, 0, 0, 1]);
        assert!(MClientMetrics::decode_payload(1, b.freeze(), Bytes::new(), Bytes::new()).is_err());
    }
}
