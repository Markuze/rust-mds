//! One reserved WAL slot. Submission encodes and transfers ownership without I/O.
//! Completion advances only a contiguous OSD-confirmed prefix.

use denc::Bytes;
use engine::{Journal, JournalError, JournalRecord, RecordPermit};

#[derive(Debug)]
pub struct Write {
    pub seq: u64,
    pub body: Bytes,
}

pub struct DaemonJournal {
    submitted: u64,
    committed: u64,
    trimmed: u64,
    max_body: usize,
    max_keys: u64,
    reserved: bool,
    enabled: bool,
    failed: Option<JournalError>,
    queued: Option<Write>,
    in_flight: bool,
    flush_requested: bool,
}

impl DaemonJournal {
    pub fn closed(covered: u64, limits: &store::Limits) -> Self {
        Self {
            submitted: covered,
            committed: covered,
            trimmed: 0,
            max_body: limits.max_record_bytes - store::framing::OVERHEAD,
            max_keys: limits
                .max_wal_keys
                .min((limits.max_wal_bytes / limits.max_record_bytes) as u64),
            reserved: false,
            enabled: false,
            failed: None,
            queued: None,
            in_flight: false,
            flush_requested: false,
        }
    }
    pub fn enable(&mut self, tail: u64, trimmed: u64) -> Result<(), JournalError> {
        if self.reserved
            || self.in_flight
            || self.queued.is_some()
            || tail < self.submitted
            || trimmed > tail
        {
            return Err(JournalError::Corrupt);
        }
        self.submitted = tail;
        self.committed = tail;
        self.trimmed = trimmed;
        self.enabled = true;
        Ok(())
    }
    pub fn reserve(&mut self) -> Option<RecordPermit> {
        if !self.enabled
            || self.failed.is_some()
            || self.reserved
            || self.in_flight
            || self.queued.is_some()
            || self.submitted.saturating_sub(self.trimmed) >= self.max_keys
        {
            return None;
        }
        self.reserved = true;
        Some(RecordPermit {
            max_record_bytes: self.max_body,
        })
    }
    pub fn release(&mut self, permit: Option<RecordPermit>) {
        if permit.is_some() {
            self.reserved = false;
        }
    }
    pub fn take_write(&mut self) -> Option<Write> {
        let write = self.queued.take()?;
        self.in_flight = true;
        Some(write)
    }
    pub fn complete(&mut self, seq: u64) -> Result<(), JournalError> {
        if self.failed.is_some()
            || !self.in_flight
            || seq != self.submitted
            || seq != self.committed + 1
        {
            return Err(JournalError::Corrupt);
        }
        self.committed = seq;
        self.in_flight = false;
        self.flush_requested = false;
        Ok(())
    }
    pub fn fail(&mut self, error: JournalError) {
        self.failed = Some(error);
        self.queued = None;
        self.reserved = false;
        self.in_flight = false;
        self.enabled = false;
    }
    pub fn trimmed(&mut self, through: u64) -> Result<(), JournalError> {
        if through < self.trimmed || through > self.committed {
            return Err(JournalError::Corrupt);
        }
        self.trimmed = through;
        Ok(())
    }
    pub fn retained(&self) -> u64 {
        self.submitted - self.trimmed
    }
    pub fn busy(&self) -> bool {
        self.in_flight || self.queued.is_some() || self.reserved
    }
    pub fn max_keys(&self) -> u64 {
        self.max_keys
    }
}

impl Journal for DaemonJournal {
    fn submit(&mut self, record: JournalRecord) -> Result<u64, JournalError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if !self.enabled {
            return Err(JournalError::Unavailable);
        }
        if !self.reserved || self.queued.is_some() || self.in_flight {
            return Err(JournalError::Capacity);
        }
        self.reserved = false;
        let body = store::record::encode_record_bounded(&record, self.max_body)
            .map_err(|_| JournalError::RecordTooLarge)?;
        let seq = self.submitted.checked_add(1).ok_or(JournalError::Corrupt)?;
        self.submitted = seq;
        self.queued = Some(Write { seq, body });
        Ok(seq)
    }
    fn committed_seq(&self) -> u64 {
        self.committed
    }
    fn flush(&mut self) {
        self.flush_requested = true;
    }
}

/// Used only to construct the deterministic initial materialized bootstrap seed.
/// No seed record can enter the production journal or receive a client reply.
pub struct SeedJournal(u64);
impl SeedJournal {
    pub fn new() -> Self {
        Self(0)
    }
}
impl Journal for SeedJournal {
    fn submit(&mut self, _: JournalRecord) -> Result<u64, JournalError> {
        self.0 += 1;
        Ok(self.0)
    }
    fn committed_seq(&self) -> u64 {
        self.0
    }
    fn flush(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reservation_recovered_sequence_and_completion_are_not_submission() {
        let limits = store::Limits::default();
        let mut journal = DaemonJournal::closed(7, &limits);
        assert!(journal.reserve().is_none());
        journal.enable(9, 7).unwrap();
        assert!(
            journal
                .submit(JournalRecord::new(engine::journal::RecordKind::Setattr))
                .is_err()
        );
        let _permit = journal.reserve().unwrap();
        assert!(journal.reserve().is_none());
        assert_eq!(
            journal
                .submit(JournalRecord::new(engine::journal::RecordKind::Setattr))
                .unwrap(),
            10
        );
        assert_eq!(journal.committed_seq(), 9);
        let write = journal.take_write().unwrap();
        assert_eq!(write.seq, 10);
        assert!(journal.reserve().is_none());
        journal.complete(10).unwrap();
        assert_eq!(journal.committed_seq(), 10);
        assert!(journal.reserve().is_some());
        journal.fail(JournalError::Fenced);
        assert!(journal.take_write().is_none());
        assert!(journal.complete(10).is_err());
        assert!(journal.reserve().is_none());
    }
}
