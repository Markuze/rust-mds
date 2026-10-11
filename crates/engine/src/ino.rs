//! Inode-number allocation from range leases.
//!
//! Rules (design 4.3 "Inode numbers", F-STORE-046/-069/-070): numbers are
//! allocated from range leases, are never reused (NFS handles carry no
//! generation number), never fall in the MDS-private range `[0x100, 0x1000)`
//! (the kernel refuses them with `EREMOTEIO`, `ceph_vino_is_reserved`) and stay
//! below 2^48 (ceph-fuse). The first lease of a new file system is the C++
//! rank-0 `InoTable` range `[1 << 40, 2 << 40)` (`InoTable::reset_state`).
//!
//! The allocator can also carve contiguous sub-ranges out of its free space
//! and delegate them to a client session (the hook for P1 kernel async
//! creates, which use `openc_response_t::delegated_inos`). Delegated numbers
//! are never handed out by `alloc`, and when a session goes away its unused
//! delegated ranges are returned to the caller to be purged, not reused.

use std::collections::{BTreeMap, VecDeque};

use crate::consts::{INO_LIMIT, MDS_INO_SYSTEM_BASE, RANK0_INO_LEN, RANK0_INO_START};
use crate::types::{ClientId, Ino, InoRange};

/// Why a lease was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InoLeaseError {
    /// The range is empty.
    Empty,
    /// The range reaches into `[0x100, 0x1000)` or below.
    Reserved,
    /// The range reaches 2^48 or beyond.
    TooLarge,
    /// The range starts below the end of an earlier lease (would allow reuse).
    Overlap,
}

/// Serializable allocator state, journaled with records that change it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InoAllocState {
    pub free: Vec<InoRange>,
    pub delegated: Vec<(ClientId, InoRange)>,
    pub floor: Ino,
}

/// The per-shard inode-number allocator.
#[derive(Debug, Clone, Default)]
pub struct InoAllocator {
    /// Unused leased ranges in ascending order; allocation takes from the front.
    free: VecDeque<InoRange>,
    /// Ranges handed to sessions and not yet used by them.
    delegated: BTreeMap<ClientId, Vec<InoRange>>,
    /// End of the highest lease ever added: later leases must start here or above.
    floor: Ino,
}

impl InoAllocator {
    /// An allocator without any lease.
    pub fn new() -> InoAllocator {
        InoAllocator::default()
    }

    /// An allocator holding the C++ rank-0 range `[1 << 40, 2 << 40)`.
    pub fn rank0() -> InoAllocator {
        let mut a = InoAllocator::new();
        a.add_lease(InoRange::new(RANK0_INO_START, RANK0_INO_LEN))
            .expect("rank-0 range is valid");
        a
    }

    /// Add a leased range. Leases must be valid inode numbers and must not
    /// overlap anything leased before (numbers are never reused).
    pub fn add_lease(&mut self, r: InoRange) -> Result<(), InoLeaseError> {
        if r.is_empty() {
            return Err(InoLeaseError::Empty);
        }
        // Everything below the system base is either a special inode (1..4) or
        // the MDS-private range [0x100, 0x1000) (`MDS_INO_MDSDIR_OFFSET`..).
        if r.start < MDS_INO_SYSTEM_BASE {
            return Err(InoLeaseError::Reserved);
        }
        if r.end() > INO_LIMIT || r.end() < r.start {
            return Err(InoLeaseError::TooLarge);
        }
        if r.start < self.floor {
            return Err(InoLeaseError::Overlap);
        }
        self.floor = r.end();
        self.free.push_back(r);
        Ok(())
    }

    /// Allocate the lowest free number, or `None` when every lease is used up
    /// (the caller then needs a new lease; operations fail with `ENOSPC`).
    pub fn alloc(&mut self) -> Option<Ino> {
        loop {
            let front = self.free.front_mut()?;
            if front.is_empty() {
                self.free.pop_front();
                continue;
            }
            let ino = front.start;
            front.start += 1;
            front.len -= 1;
            if front.is_empty() {
                self.free.pop_front();
            }
            return Some(ino);
        }
    }

    /// Number of numbers still available to `alloc`.
    pub fn free_count(&self) -> u64 {
        self.free.iter().map(|r| r.len).sum()
    }

    /// Whether `ino` is still free (neither allocated nor delegated).
    pub fn is_free(&self, ino: Ino) -> bool {
        self.free.iter().any(|r| r.contains(ino))
    }

    /// Remove a specific number from the free space (journal replay of an
    /// allocation, or a client-chosen number). Returns false if it was not free.
    pub fn take(&mut self, ino: Ino) -> bool {
        let Some(idx) = self.free.iter().position(|r| r.contains(ino)) else {
            return false;
        };
        let r = self.free[idx];
        let left = InoRange::new(r.start, ino - r.start);
        let right = InoRange::new(ino + 1, r.end() - ino - 1);
        self.free.remove(idx);
        if !right.is_empty() {
            self.free.insert(idx, right);
        }
        if !left.is_empty() {
            self.free.insert(idx, left);
        }
        true
    }

    /// Delegate up to `count` contiguous numbers to `client` (P1 async
    /// creates). The range comes from the front of the free space.
    pub fn delegate(&mut self, client: ClientId, count: u64) -> Option<InoRange> {
        if count == 0 {
            return None;
        }
        let front = self.free.front_mut()?;
        let n = count.min(front.len);
        let r = InoRange::new(front.start, n);
        front.start += n;
        front.len -= n;
        if front.is_empty() {
            self.free.pop_front();
        }
        self.delegated.entry(client).or_default().push(r);
        Some(r)
    }

    /// The client used one of its delegated numbers. Returns false if `ino`
    /// was not delegated to it (the caller must then refuse the number).
    pub fn take_delegated(&mut self, client: ClientId, ino: Ino) -> bool {
        let Some(ranges) = self.delegated.get_mut(&client) else {
            return false;
        };
        let Some(idx) = ranges.iter().position(|r| r.contains(ino)) else {
            return false;
        };
        let r = ranges.remove(idx);
        let left = InoRange::new(r.start, ino - r.start);
        let right = InoRange::new(ino + 1, r.end() - ino - 1);
        if !left.is_empty() {
            ranges.push(left);
        }
        if !right.is_empty() {
            ranges.push(right);
        }
        ranges.sort();
        if ranges.is_empty() {
            self.delegated.remove(&client);
        }
        true
    }

    /// Ranges currently delegated to `client`.
    pub fn delegated_to(&self, client: ClientId) -> &[InoRange] {
        self.delegated
            .get(&client)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Drop the delegations of a session that went away. The returned ranges
    /// are never reused; their data objects may need purging.
    pub fn revoke_delegations(&mut self, client: ClientId) -> Vec<InoRange> {
        self.delegated.remove(&client).unwrap_or_default()
    }

    /// State for a journal record.
    pub fn state(&self) -> InoAllocState {
        InoAllocState {
            free: self.free.iter().copied().collect(),
            delegated: self
                .delegated
                .iter()
                .flat_map(|(c, v)| v.iter().map(move |r| (*c, *r)))
                .collect(),
            floor: self.floor,
        }
    }

    /// Restore from a journaled state.
    pub fn restore(state: &InoAllocState) -> InoAllocator {
        let mut delegated: BTreeMap<ClientId, Vec<InoRange>> = BTreeMap::new();
        for (c, r) in &state.delegated {
            delegated.entry(*c).or_default().push(*r);
        }
        InoAllocator {
            free: state.free.iter().copied().collect(),
            delegated,
            floor: state.floor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank0_starts_at_cxx_range() {
        let mut a = InoAllocator::rank0();
        assert_eq!(a.alloc(), Some(0x100_0000_0000));
        assert_eq!(a.alloc(), Some(0x100_0000_0001));
        assert_eq!(a.free_count(), RANK0_INO_LEN - 2);
    }

    #[test]
    fn leases_are_validated() {
        let mut a = InoAllocator::new();
        assert_eq!(
            a.add_lease(InoRange::new(0x100, 10)),
            Err(InoLeaseError::Reserved)
        );
        assert_eq!(
            a.add_lease(InoRange::new(0xff0, 0x20)),
            Err(InoLeaseError::Reserved)
        );
        assert_eq!(
            a.add_lease(InoRange::new(INO_LIMIT - 1, 2)),
            Err(InoLeaseError::TooLarge)
        );
        assert_eq!(
            a.add_lease(InoRange::new(0x2000, 0)),
            Err(InoLeaseError::Empty)
        );
        assert!(a.add_lease(InoRange::new(0x2000, 2)).is_ok());
        assert_eq!(
            a.add_lease(InoRange::new(0x2001, 4)),
            Err(InoLeaseError::Overlap)
        );
        assert!(a.add_lease(InoRange::new(0x3000, 1)).is_ok());
        assert_eq!(a.alloc(), Some(0x2000));
        assert_eq!(a.alloc(), Some(0x2001));
        assert_eq!(a.alloc(), Some(0x3000));
        assert_eq!(a.alloc(), None, "exhausted, no reuse");
    }

    #[test]
    fn take_splits_ranges() {
        let mut a = InoAllocator::new();
        a.add_lease(InoRange::new(0x2000, 10)).unwrap();
        assert!(a.take(0x2004));
        assert!(!a.take(0x2004));
        assert!(!a.is_free(0x2004));
        assert_eq!(a.free_count(), 9);
        let got: Vec<_> = std::iter::from_fn(|| a.alloc()).collect();
        assert_eq!(got.len(), 9);
        assert!(!got.contains(&0x2004));
    }

    #[test]
    fn delegation_never_overlaps_alloc() {
        let mut a = InoAllocator::new();
        a.add_lease(InoRange::new(0x2000, 100)).unwrap();
        let first = a.alloc().unwrap();
        let d = a.delegate(7, 10).unwrap();
        assert_eq!(d, InoRange::new(first + 1, 10));
        let next = a.alloc().unwrap();
        assert!(!d.contains(next));
        assert!(a.take_delegated(7, d.start + 3));
        assert!(!a.take_delegated(7, d.start + 3));
        assert!(!a.take_delegated(8, d.start));
        assert_eq!(a.delegated_to(7).iter().map(|r| r.len).sum::<u64>(), 9);
        let back = a.revoke_delegations(7);
        assert_eq!(back.iter().map(|r| r.len).sum::<u64>(), 9);
        // Revoked numbers are not reusable.
        assert!(!a.is_free(d.start));
    }

    #[test]
    fn state_roundtrip() {
        let mut a = InoAllocator::rank0();
        a.alloc();
        a.delegate(3, 5);
        let s = a.state();
        let mut b = InoAllocator::restore(&s);
        assert_eq!(b.state(), s);
        assert_eq!(a.alloc(), b.alloc());
    }
}
