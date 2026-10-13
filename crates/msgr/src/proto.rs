//! The boundary between the session layer ([`crate::conn`]) and a wire protocol. msgr1
//! ([`crate::v1`]) implements it today; msgr2 will implement the same traits (its frames carry
//! the same session events: messages with sequence numbers, acks, keepalives).

use std::future::Future;

use denc::{EntityName, RawMessage, UTime};

use crate::MsgrError;

/// A frame received on an established session.
#[derive(Debug)]
pub(crate) enum InFrame {
    /// A message with its session sequence number; `msg.src` is set.
    Message {
        seq: u64,
        msg: RawMessage,
    },
    /// The peer received everything up to this sequence number.
    Ack(u64),
    Keepalive,
    /// Keepalive with the peer's timestamp, to be echoed.
    Keepalive2(UTime),
    /// Echo of one of our keepalives.
    Keepalive2Ack(UTime),
    /// The peer closes the session for good.
    Close,
}

/// A frame to send on an established session.
#[derive(Debug, Clone)]
pub(crate) enum OutFrame {
    Message {
        seq: u64,
        msg: RawMessage,
    },
    Ack(u64),
    Keepalive,
    Keepalive2(UTime),
    Keepalive2Ack(UTime),
    #[allow(dead_code)]
    Close,
}

/// Reading half of an established socket.
pub(crate) trait FrameReader: Send + 'static {
    fn read_frame(&mut self) -> impl Future<Output = Result<InFrame, MsgrError>> + Send;
}

/// Writing half of an established socket. `src` is the messenger's current name.
pub(crate) trait FrameWriter: Send + 'static {
    fn write_frames(
        &mut self,
        frames: &[OutFrame],
        src: EntityName,
    ) -> impl Future<Output = Result<(), MsgrError>> + Send;
}
