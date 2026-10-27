//! Bounded daemon input ownership. Engine reservations are a separate resource.
use super::*;

#[derive(Clone, PartialEq, Eq)]
enum Identity {
    Request(SessionKey, u64, u32),
    Caps(SessionKey, u64, u64),
    Release(SessionKey),
    Close(SessionKey),
    Job(u64),
    Tick,
    Marker,
}
impl Identity {
    fn of(input: &EngineInput) -> Self {
        match input {
            EngineInput::Request { key, request } => {
                Self::Request(*key, request.tid, request.op.code())
            }
            EngineInput::CapUpdate { key, update } => {
                Self::Caps(*key, update.ino, update.flush_tid)
            }
            EngineInput::CapRelease { key, .. } => Self::Release(*key),
            EngineInput::CloseSession { key, .. } => Self::Close(*key),
            EngineInput::LifecycleCompleted { id, .. } => Self::Job(*id),
            EngineInput::Tick => Self::Tick,
            EngineInput::RecoveryControlsDrained => Self::Marker,
        }
    }
    fn key(&self) -> Option<SessionKey> {
        match self {
            Self::Request(k, ..) | Self::Caps(k, ..) | Self::Release(k) | Self::Close(k) => {
                Some(*k)
            }
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(super) struct Origin {
    identity: Identity,
    pub serial: u64,
    pub ordinal: u64,
    pub received: Instant,
    pub bytes: usize,
    pub control: bool,
    pub owner: Option<crate::session::OutputOwner>,
}
pub(super) struct Pressure {
    pub input: Input,
    pub epoch: u64,
}

impl Runtime {
    pub(super) fn observe_epoch(&mut self, epoch: u64) {
        if self.capacity_epoch != epoch {
            self.capacity_epoch = epoch;
            // Per-waiter stamps retain a finite pass across scheduler yields.
        }
    }
    pub(super) fn pressure_ready(&self) -> bool {
        self.retry_wait
            .iter()
            .any(|p| p.epoch != self.capacity_epoch)
    }
    pub(super) fn release_origin(&mut self, origin: Origin) {
        if origin.control {
            self.control_bytes -= origin.bytes;
        } else {
            self.normal_bytes -= origin.bytes;
        }
    }
    pub(super) fn returned_origin(&mut self, input: &EngineInput) -> Option<Origin> {
        let identity = Identity::of(input);
        let serial = self
            .owned
            .iter()
            .find_map(|(serial, o)| (o.identity == identity).then_some(*serial))?;
        self.owned.remove(&serial)
    }
    pub(super) fn reply_origin(&mut self, client: u64, tid: u64) {
        let serial = self.owned.iter().find_map(|(serial, o)| {
            matches!(o.identity, Identity::Request(k, t, _) if k.client == client && t == tid)
                .then_some(*serial)
        });
        if let Some(serial) = serial
            && let Some(origin) = self.owned.remove(&serial)
        {
            self.release_origin(origin);
        }
    }
    pub(super) fn finish_controls(&mut self) {
        if !matches!(self.state, StepState::Idle | StepState::WaitingInput) {
            return;
        }
        let done: Vec<_> = self
            .owned
            .iter()
            .filter_map(|(id, o)| {
                (!matches!(o.identity, Identity::Request(..)) && !self.output_owned.contains(id))
                    .then_some(*id)
            })
            .collect();
        for id in done {
            if let Some(origin) = self.owned.remove(&id) {
                self.release_origin(origin);
            }
        }
    }
    pub(super) fn control_owner(&self) -> Option<crate::session::OutputOwner> {
        self.owned
            .values()
            .find(|o| !matches!(o.identity, Identity::Request(..)))?
            .owner
            .clone()
    }
    pub(super) fn control_serial(&self) -> Option<u64> {
        self.owned
            .iter()
            .find_map(|(id, o)| (!matches!(o.identity, Identity::Request(..))).then_some(*id))
    }
    pub(super) fn new_origin(
        &mut self,
        input: &EngineInput,
        bytes: usize,
        control: bool,
    ) -> Result<Origin, ()> {
        self.next_input = self.next_input.checked_add(1).ok_or(())?;
        Ok(Origin {
            identity: Identity::of(input),
            serial: self.next_input,
            ordinal: self.receive_ordinal.take().unwrap_or(self.next_input),
            received: Instant::now(),
            bytes,
            control,
            owner: None,
        })
    }
    pub(super) fn closed_origins(&mut self, key: SessionKey) {
        let ids: Vec<_> = self
            .owned
            .iter()
            .filter_map(|(id, o)| (o.identity.key() == Some(key)).then_some(*id))
            .collect();
        for id in ids {
            if let Some(origin) = self.owned.remove(&id) {
                self.release_origin(origin);
            }
        }
    }
}
