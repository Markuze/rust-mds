//! One-shot suppression of a committed client send.
//!
//! The journal record is already durable before `Server::send_effects` runs.
//! This seam drops only the matching reply or `FLUSH_ACK`. A later retry of
//! the same tid must be answered from the completed-request table.
//!
//! The daemon listens only when `RMDS_SEND_FAULT_SOCK` names a datagram
//! socket path. Each datagram is one ASCII command:
//!
//! `reply <gid> <client> <incarnation> <tid>`
//!
//! `flush_ack <gid> <client> <incarnation> <tid>`
//!
//! `<gid>` is the MDS global id. `<client>` and `<incarnation>` are the
//! durable session key. A command replaces any fault that is still armed.

use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use engine::SessionKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SendFaultKind {
    Reply,
    FlushAck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SendFault {
    pub gid: u64,
    pub session: SessionKey,
    pub tid: u64,
    pub kind: SendFaultKind,
}

pub(crate) fn parse_send_fault(text: &str) -> Option<SendFault> {
    let mut parts = text.split_whitespace();
    let kind = match parts.next()? {
        "reply" => SendFaultKind::Reply,
        "flush_ack" => SendFaultKind::FlushAck,
        _ => return None,
    };
    let gid = u64::from_str(parts.next()?).ok()?;
    let client = u64::from_str(parts.next()?).ok()?;
    let incarnation = u64::from_str(parts.next()?).ok()?;
    let tid = u64::from_str(parts.next()?).ok()?;
    if parts.next().is_some() || client == 0 {
        return None;
    }
    Some(SendFault {
        gid,
        session: SessionKey {
            client,
            incarnation,
        },
        tid,
        kind,
    })
}

pub(crate) fn listen(
    path: impl AsRef<Path>,
    tx: tokio::sync::mpsc::Sender<SendFault>,
) -> std::io::Result<()> {
    let path = PathBuf::from(path.as_ref());
    if path.as_os_str().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "empty send-fault socket path",
        ));
    }
    if let Ok(meta) = std::fs::symlink_metadata(&path) {
        if !meta.file_type().is_socket() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "send-fault path exists and is not a socket",
            ));
        }
        std::fs::remove_file(&path)?;
    }
    let socket = std::os::unix::net::UnixDatagram::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let cleanup = path.clone();
    std::thread::Builder::new()
        .name("rmds-send-fault".into())
        .spawn(move || {
            let mut buf = [0u8; 128];
            loop {
                let Ok(n) = socket.recv(&mut buf) else {
                    break;
                };
                let text = String::from_utf8_lossy(&buf[..n]);
                let text = text.trim();
                match parse_send_fault(text) {
                    Some(fault) => {
                        tracing::info!(?fault, "armed send fault");
                        if tx.blocking_send(fault).is_err() {
                            break;
                        }
                    }
                    None => tracing::warn!(command = text, "rejected send-fault command"),
                }
            }
            let _ = std::fs::remove_file(cleanup);
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_one_reply_or_flush_and_rejects_the_rest() {
        let reply = parse_send_fault("reply 9 4100 3 7").unwrap();
        assert_eq!(reply.gid, 9);
        assert_eq!(reply.session.client, 4100);
        assert_eq!(reply.session.incarnation, 3);
        assert_eq!(reply.tid, 7);
        assert_eq!(reply.kind, SendFaultKind::Reply);
        assert_eq!(
            parse_send_fault("flush_ack 1 2 0 44").unwrap().kind,
            SendFaultKind::FlushAck
        );
        assert!(parse_send_fault("reply 9 4100 3").is_none());
        assert!(parse_send_fault("reply 9 4100 3 7 extra").is_none());
        assert!(parse_send_fault("reply 9 0 3 7").is_none());
        assert!(parse_send_fault("grant 9 4100 3 7").is_none());
    }
}
