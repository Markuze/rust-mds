//! The engine's per-client session state: the caps the client holds, the
//! inodes it has file locks on, and the idempotency tables (completed
//! requests and cap flushes) that are journaled with the mutations.
//!
//! The session *protocol* (open, renew, close, push_seq) belongs to the
//! daemon; the engine is told when a session opens, goes stale, resumes or
//! closes.

use std::collections::{BTreeMap, BTreeSet};

use crate::journal::CompletedRequest;
use crate::types::{ClientId, Ino};

/// Session liveness as far as caps are concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Open,
    /// Missed renewals: its caps are ignored for policy and are revoked
    /// forcibly (or the client is evicted if it holds write caps).
    Stale,
}

/// Per-client state.
#[derive(Debug, Clone)]
pub struct Session {
    pub client: ClientId,
    /// Incarnation of the session: a client that closes and opens again gets
    /// a new epoch, and results of the old incarnation's requests are dropped.
    pub epoch: u64,
    pub state: SessionState,
    /// Cap generation; bumped when the session goes stale. Caps of an older
    /// generation are invalid until re-issued.
    pub cap_gen: u32,
    /// Inodes on which the client holds a cap.
    pub caps: BTreeSet<Ino>,
    /// Inodes on which the client holds or waits for file locks.
    pub lock_inos: BTreeSet<Ino>,
    /// Successful mutating requests by tid.
    pub completed_requests: BTreeMap<u64, CompletedRequest>,
    /// Cap flush tids already applied and acknowledged.
    pub completed_flushes: BTreeSet<u64>,
}

impl Session {
    pub fn new(client: ClientId) -> Session {
        Session::with_epoch(client, 0)
    }

    pub fn with_epoch(client: ClientId, epoch: u64) -> Session {
        Session {
            client,
            epoch,
            state: SessionState::Open,
            cap_gen: 0,
            caps: BTreeSet::new(),
            lock_inos: BTreeSet::new(),
            completed_requests: BTreeMap::new(),
            completed_flushes: BTreeSet::new(),
        }
    }

    pub fn is_stale(&self) -> bool {
        self.state == SessionState::Stale
    }

    pub fn num_caps(&self) -> usize {
        self.caps.len()
    }

    /// Drop completed requests the client no longer retries
    /// (`trim_completed_request_list(oldest_client_tid)`).
    pub fn trim_completed_requests(&mut self, oldest_client_tid: u64) {
        if oldest_client_tid > 0 {
            self.completed_requests = self.completed_requests.split_off(&oldest_client_tid);
        }
    }

    /// Drop completed flushes below the client's oldest pending flush tid.
    pub fn trim_completed_flushes(&mut self, oldest_flush_tid: u64) {
        if oldest_flush_tid > 0 {
            self.completed_flushes = self.completed_flushes.split_off(&oldest_flush_tid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trimming() {
        let mut s = Session::new(4);
        for t in 1..=5 {
            s.completed_requests.insert(
                t,
                CompletedRequest {
                    client: 4,
                    tid: t,
                    op: 0,
                    result: 0,
                    created_ino: None,
                    oldest_client_tid: 0,
                },
            );
            s.completed_flushes.insert(t);
        }
        s.trim_completed_requests(3);
        assert_eq!(
            s.completed_requests.keys().copied().collect::<Vec<_>>(),
            vec![3, 4, 5]
        );
        s.trim_completed_flushes(5);
        assert_eq!(
            s.completed_flushes.iter().copied().collect::<Vec<_>>(),
            vec![5]
        );
        s.trim_completed_requests(0);
        assert_eq!(s.completed_requests.len(), 3);
    }
}
