//! Collection-only recovery. C++ references: Server::handle_client_reconnect,
//! MDSDaemon::ms_handle_accept and MDCache::finish_snaprealm_reconnect.
//! Nonempty finalization requires the separately owned E2 engine grant gate.

use super::*;
use denc::{EntityAddrVec, Message};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_RECOVERY_BYTES: usize = 32 << 20;
pub const MAX_RECOVERY_INPUTS: usize = 1024;
const MAX_RECOVERY_CHUNKS: usize = 128;
pub const MAX_RECONNECT_MESSAGE: usize = 1 << 20;
const MAX_CAPS: usize = 16384;
const MAX_LOCKS: usize = 65536;
const MAX_REALMS: usize = 4096;
const MAX_ADDRESSES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Token {
    connection: u64,
    generation: u64,
}
pub(super) enum OpenPurpose {
    Ordinary,
    Recovery { key: SessionKey, token: Token },
}

struct Attachment {
    link: Box<dyn ClientLink>,
    auths: Vec<MdsCapAuth>,
    token: Token,
    bytes: usize,
    features: u64,
}
struct Held {
    key: Option<SessionKey>,
    token: Token,
    msg: ClientMessage,
    bytes: usize,
    received: Instant,
    activity_noted: bool,
}
#[derive(Clone)]
struct AppliedCap {
    realm: u64,
    locks: usize,
    bytes: usize,
}
struct Member {
    binding: Option<(denc::EntityAddr, Bytes, u64)>,
    token: Option<Token>,
    generation: u64,
    disconnected: bool,
    attachment: Option<Attachment>,
    attached: bool,
    complete: bool,
    final_token: Option<Token>,
    evicting: bool,
    close_reply: bool,
    caps: BTreeMap<u64, AppliedCap>,
    realms: BTreeMap<u64, engine::ReconnectRealm>,
    correction_bytes: usize,
}
impl Member {
    fn new() -> Self {
        Self {
            binding: None,
            token: None,
            generation: 0,
            disconnected: false,
            attachment: None,
            attached: false,
            complete: false,
            final_token: None,
            evicting: false,
            close_reply: false,
            caps: BTreeMap::new(),
            realms: BTreeMap::new(),
            correction_bytes: 0,
        }
    }
    fn bytes(&self) -> usize {
        512 + self.binding.as_ref().map_or(0, |(_, ctx, _)| ctx.len())
            + self.caps.values().map(|c| c.bytes).sum::<usize>()
            + self.realms.len() * 96
            + self.correction_bytes
            + self.attachment.as_ref().map_or(0, |a| a.bytes)
    }
}
struct Window {
    start: Instant,
    last: Instant,
    timeout: Duration,
    expired: bool,
}
impl Window {
    fn deadline(&self) -> Instant {
        (self.start + self.timeout * 2)
            .min((self.start + self.timeout).max(self.last + self.timeout / 2))
    }
}
pub(super) struct Recovery {
    /// Sticky for this load, even if every original member later reaches Closed.
    pub nonempty: bool,
    confirmed: bool,
    collection_allowed: bool,
    window: Option<Window>,
    members: BTreeMap<SessionKey, Member>,
    chunks: VecDeque<Held>,
    early: VecDeque<Held>,
    input_bytes: usize,
}
impl Recovery {
    pub fn new(keys: &BTreeSet<SessionKey>) -> Self {
        Self {
            nonempty: !keys.is_empty(),
            confirmed: false,
            collection_allowed: false,
            window: None,
            members: keys.iter().map(|k| (*k, Member::new())).collect(),
            chunks: VecDeque::new(),
            early: VecDeque::new(),
            input_bytes: 0,
        }
    }
    fn bytes(&self) -> usize {
        self.input_bytes + self.members.values().map(Member::bytes).sum::<usize>()
    }
    pub fn runnable(&self, opening: &HashMap<u64, Opening>) -> bool {
        self.members.iter().any(|(key, m)| {
            !m.evicting && m.attachment.is_some() && !opening.contains_key(&key.client)
        }) || (self.confirmed
            && self.collection_allowed
            && self
                .chunks
                .iter()
                .chain(self.early.iter().filter(|h| !h.activity_noted))
                .any(|h| {
                    h.key.and_then(|k| self.members.get(&k)).is_some_and(|m| {
                        m.attached && !m.evicting && !m.disconnected && m.token == Some(h.token)
                    })
                }))
    }
    pub fn closed(&mut self, key: SessionKey) {
        self.members.remove(&key);
        self.chunks.retain(|h| {
            if h.key == Some(key) {
                self.input_bytes -= h.bytes;
                false
            } else {
                true
            }
        });
        self.early.retain(|h| {
            if h.key == Some(key) {
                self.input_bytes -= h.bytes;
                false
            } else {
                true
            }
        });
    }
    fn reset_partial(&mut self, key: SessionKey) {
        if let Some(m) = self.members.get_mut(&key) {
            m.caps.clear();
            m.realms.clear();
            m.correction_bytes = 0;
        }
        self.chunks.retain(|h| {
            if h.key == Some(key) {
                self.input_bytes -= h.bytes;
                false
            } else {
                true
            }
        });
        // Early controls are not a completed drain. Retain them for D5's checked
        // replacement reconstruction or the key's fenced durable cleanup.
    }
}

impl Runtime {
    pub(in crate::server) fn freeze_recovery_client(&mut self, client: u64) -> bool {
        if !self.recovery.nonempty {
            return false;
        }
        let Some(key) = self.keys.get(&client) else {
            return false;
        };
        let Some(member) = self.recovery.members.get_mut(key) else {
            return false;
        };
        member.evicting = true;
        member.attachment = None;
        true
    }
}

/// A durable key and a frozen complete address union, not a fabricated connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryFence {
    pub owner: OwnerToken,
    pub key: SessionKey,
    pub attachment: Option<(u64, u64)>,
    pub addresses: EntityAddrVec,
}
#[derive(Debug, Clone, Copy)]
pub struct RecoveryStatus {
    pub missing: usize,
    pub surviving: usize,
    pub complete: usize,
    pub caps: usize,
    pub locks: usize,
    pub realms: usize,
    pub bytes: usize,
    pub early_inputs: usize,
    pub finalization_blocked: bool,
}

impl Server<DaemonJournal> {
    pub fn recovery_status(&self) -> Option<RecoveryStatus> {
        let r = self.runtime.as_ref()?;
        Some(RecoveryStatus {
            missing: r.gather.len(),
            surviving: r.recovery.members.len(),
            complete: r.recovery.members.values().filter(|m| m.complete).count(),
            caps: r.recovery.members.values().map(|m| m.caps.len()).sum(),
            locks: r
                .recovery
                .members
                .values()
                .flat_map(|m| m.caps.values())
                .map(|c| c.locks)
                .sum(),
            realms: r.recovery.members.values().map(|m| m.realms.len()).sum(),
            bytes: r.recovery.bytes() + r.opening_bytes,
            early_inputs: r.recovery.early.len(),
            finalization_blocked: r.recovery.nonempty && !r.recovered,
        })
    }
    pub fn recovery_pending(&self) -> bool {
        self.runtime
            .as_ref()
            .is_some_and(|r| r.recovery.nonempty && !r.recovered)
    }
    pub fn pause_execution(&mut self, paused: bool) {
        if let Some(r) = &mut self.runtime {
            r.execution_paused = paused;
        }
    }
    pub fn set_runtime_permissions(&mut self, ordinary: bool, data: bool, execution: bool) {
        self.work_paused = !data;
        self.pause_execution(!execution);
        if let Some(r) = &mut self.runtime {
            r.pause_ordinary(!ordinary);
        }
    }
    pub fn allow_recovery_collection(&mut self, allowed: bool) {
        if let Some(r) = &mut self.runtime {
            r.recovery.collection_allowed = allowed;
        }
    }
    pub fn confirm_reconnect(&mut self, now: Instant, timeout: Duration) {
        if timeout.is_zero()
            || timeout
                .checked_mul(2)
                .and_then(|d| now.checked_add(d))
                .is_none()
        {
            self.fail_runtime(JournalError::Corrupt, "invalid reconnect deadline");
            return;
        }
        if let Some(r) = &mut self.runtime {
            r.recovery.confirmed = true;
            r.recovery.collection_allowed = true;
            if r.recovery.nonempty && r.recovery.window.is_none() {
                r.recovery.window = Some(Window {
                    start: now,
                    last: now,
                    timeout,
                    expired: false,
                });
            }
        }
    }
    pub fn reconnect_deadline(&self) -> Option<Instant> {
        self.runtime.as_ref().and_then(|r| {
            r.recovery
                .window
                .as_ref()
                .filter(|w| !w.expired && !r.gather.is_empty())
                .map(Window::deadline)
        })
    }
    pub fn expire_reconnect(&mut self, sessions: &mut SessionMap, now: Instant) {
        let Some(r) = &mut self.runtime else {
            return;
        };
        let Some(w) = &mut r.recovery.window else {
            return;
        };
        if w.expired || now < w.deadline() {
            return;
        }
        w.expired = true;
        for key in &r.gather {
            if let Some(m) = r.recovery.members.get_mut(key) {
                m.evicting = true;
                m.attachment = None;
            }
            if let Some(s) = sessions.get_mut(key.client) {
                s.state = SessionState::Fencing;
            }
        }
    }
    pub fn recovery_accepted(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
        auths: Vec<MdsCapAuth>,
    ) {
        if !self.recovery_pending() {
            return;
        }
        self.recovery_receive(sessions, client, link, None, auths, 0, Instant::now());
    }
    #[allow(clippy::too_many_arguments)] // Keep wire identity, receive time and byte accounting at the input boundary.
    pub fn recovery_input(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: ClientMessage,
        auths: Vec<MdsCapAuth>,
        bytes: usize,
        received: Instant,
    ) {
        self.recovery_receive(sessions, client, link, Some(msg), auths, bytes, received);
    }
    #[allow(clippy::too_many_arguments)] // Accepted and message events share the same checked attachment boundary.
    fn recovery_receive(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: Option<ClientMessage>,
        auths: Vec<MdsCapAuth>,
        bytes: usize,
        received: Instant,
    ) {
        if self.fatal_error().is_some() || sessions.output_failed() {
            return;
        }
        self.expire_reconnect(sessions, Instant::now());
        let Some(mut r) = self.runtime.take() else {
            return;
        };
        if let Err(reason) =
            self.stage_recovery(&mut r, sessions, client, link, msg, auths, bytes, received)
        {
            self.fail_runtime(JournalError::Capacity, reason);
            r.cancel_pending();
        }
        self.runtime = Some(r);
        self.pump(sessions, 32);
    }

    /// A current attachment's CLOSE follows its durable Closed transition.
    /// An unmatched connection has no rights and uses a non-admitted funnel.
    fn recovery_terminal(
        r: &mut Runtime,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
        reply: MClientSession,
    ) {
        if reply.op == SessionOp::CLOSE
            && link.connection_id() != 0
            && sessions
                .get(client)
                .is_some_and(|s| s.link.connection_id() == link.connection_id())
            && let Some(key) = r.keys.get(&client).copied()
        {
            r.freeze_recovery_client(client);
            if let Some(member) = r.recovery.members.get_mut(&key) {
                member.close_reply = true;
            }
            if let Some(close) = r.closing.get_mut(&key) {
                close.reply = true;
            }
            if let Some(s) = sessions.get_mut(client) {
                s.state = SessionState::Fencing;
            }
            return;
        }
        sessions.recovery_terminal(client, link, reply);
    }

    #[allow(clippy::too_many_arguments)] // Runtime is temporarily taken while this boundary validates identity and budgets.
    fn stage_recovery(
        &mut self,
        r: &mut Runtime,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: Option<ClientMessage>,
        auths: Vec<MdsCapAuth>,
        bytes: usize,
        received: Instant,
    ) -> Result<(), &'static str> {
        if bytes > MAX_RECONNECT_MESSAGE {
            return Err("recovery message exceeds 1 MiB");
        }
        let Some((name, context)) = link.authenticated_identity() else {
            return Ok(());
        };
        let connection = link.connection_id();
        if connection == 0 {
            return Err("recovery connection has no messenger identity");
        }
        let key = r.keys.get(&client).copied();
        let row = key.and_then(|k| r.rows.get(&k)).or_else(|| {
            r.rows
                .values()
                .filter(|row| row.key.client == client)
                .max_by_key(|row| row.key.incarnation)
        });
        let reconnect = matches!(&msg, Some(ClientMessage::Reconnect(_)));
        let open =
            matches!(&msg, Some(ClientMessage::Session(s)) if s.op == SessionOp::REQUEST_OPEN);
        if row.is_none_or(|row| row.lifecycle != SessionLifecycle::Open) {
            if reconnect {
                Self::recovery_terminal(
                    r,
                    sessions,
                    client,
                    link,
                    if row.is_some() {
                        MClientSession::close()
                    } else {
                        MClientSession::reject("unknown recovery session", false)
                    },
                );
            }
            // Unknown ordinary requests/OPEN stay outside the engine. Sessionless
            // renewal is ignored. A later package must revalidate held requests.
            else if let Some(msg) = msg
                && (matches!(&msg, ClientMessage::Request(_))
                    || matches!(&msg, ClientMessage::Session(s) if s.op == SessionOp::REQUEST_OPEN))
            {
                Self::hold_recovery(
                    r,
                    None,
                    Token {
                        connection,
                        generation: 0,
                    },
                    msg,
                    bytes,
                    received,
                )?;
            }
            return Ok(());
        }
        let key = key.ok_or("missing recovery key")?;
        let row = row.ok_or("missing recovery row")?;
        if row.identity.authenticated_entity != name
            || self.is_blocklisted(&link.peer_addr())
            || !self.features_allowed(row.identity.client_features)
        {
            if reconnect || open {
                Self::recovery_terminal(
                    r,
                    sessions,
                    client,
                    link,
                    if reconnect {
                        MClientSession::close()
                    } else {
                        MClientSession::reject(
                            "recovery identity or required CephFS features denied",
                            false,
                        )
                    },
                );
            }
            return Ok(());
        }
        let Some(member) = r.recovery.members.get(&key) else {
            return Ok(());
        };
        if member.evicting {
            if reconnect {
                Self::recovery_terminal(r, sessions, client, link, MClientSession::close());
            }
            return Ok(());
        }
        if member.token.is_some_and(|t| {
            connection < t.connection || (connection == t.connection && member.disconnected)
        }) {
            return Ok(());
        }
        let features = if let Some(ClientMessage::Session(msg)) = &msg
            && msg.op == SessionOp::REQUEST_OPEN
        {
            let offered = msg.supported_features.first_word();
            if !self.features_allowed(offered) {
                sessions.recovery_terminal(
                    client,
                    link,
                    MClientSession::reject("missing required CephFS features", false),
                );
                return Ok(());
            }
            offered
        } else {
            member
                .binding
                .as_ref()
                .map_or(row.identity.client_features, |(_, _, features)| *features)
        };
        let changed = member.token.is_none_or(|t| t.connection != connection)
            || member
                .binding
                .as_ref()
                .is_none_or(|(addr, ctx, old_features)| {
                    *addr != link.peer_addr() || *ctx != context || *old_features != features
                });
        if changed {
            if !member.complete {
                self.engine
                    .reset_reconnect(key)
                    .map_err(|_| "reset incomplete reconnect failed")?;
                r.recovery.reset_partial(key);
                sessions.remove(client);
                r.attached.remove(&client);
            } else {
                r.recovery.chunks.retain(|h| {
                    if h.key == Some(key) {
                        r.recovery.input_bytes -= h.bytes;
                        false
                    } else {
                        true
                    }
                });
            }
            let member = r.recovery.members.get_mut(&key).ok_or("missing member")?;
            member.generation = member
                .generation
                .checked_add(1)
                .ok_or("attachment generation exhausted")?;
            member.token = Some(Token {
                connection,
                generation: member.generation,
            });
            member.binding = Some((link.peer_addr(), context.clone(), features));
            member.disconnected = false;
            member.attached = false;
        }
        let member = r.recovery.members.get_mut(&key).ok_or("missing member")?;
        let token = member.token.ok_or("missing connection token")?;
        if changed
            || (!member.attached && member.attachment.is_none() && !r.opening.contains_key(&client))
        {
            let charge = 1024
                + name.len() * 2
                + context.len() * 2
                + row
                    .identity
                    .metadata
                    .iter()
                    .map(|(k, v)| k.len() + v.len() + 64)
                    .sum::<usize>()
                + MClientSession::open_default(0, auths.clone())
                    .to_raw(link.features())
                    .front
                    .len()
                    * 2;
            member.attachment = Some(Attachment {
                link,
                auths,
                token,
                bytes: charge,
                features,
            });
        }
        if let Some(msg) = msg {
            Self::hold_recovery(r, Some(key), token, msg, bytes, received)?;
        }
        if r.recovery.bytes().saturating_add(r.opening_bytes) > MAX_RECOVERY_BYTES {
            return Err("recovery aggregate byte budget exhausted");
        }
        Ok(())
    }
    fn hold_recovery(
        r: &mut Runtime,
        key: Option<SessionKey>,
        token: Token,
        msg: ClientMessage,
        bytes: usize,
        received: Instant,
    ) -> Result<(), &'static str> {
        let charge = bytes.saturating_mul(4).saturating_add(256);
        let chunk = matches!(msg, ClientMessage::Reconnect(_));
        if (chunk && r.recovery.chunks.len() >= MAX_RECOVERY_CHUNKS)
            || (!chunk && r.recovery.early.len() >= MAX_RECOVERY_INPUTS - MAX_RECOVERY_CHUNKS)
            || charge > MAX_RECOVERY_BYTES.saturating_sub(r.recovery.bytes() + r.opening_bytes)
        {
            return Err("recovery held input budget exhausted");
        }
        let held = Held {
            key,
            token,
            msg,
            bytes: charge,
            received,
            activity_noted: false,
        };
        r.recovery.input_bytes += charge;
        if chunk {
            r.recovery.chunks.push_back(held);
        } else {
            r.recovery.early.push_back(held);
        }
        Ok(())
    }
    pub(super) fn features_allowed(&self, features: u64) -> bool {
        self.required_client_features
            .iter()
            .enumerate()
            .all(|(i, required)| {
                let offered = if i < 8 {
                    (features >> (i * 8)) as u8
                } else {
                    0
                };
                required & !offered == 0
            })
    }
    pub(super) fn recovery_opened(
        &mut self,
        sessions: &mut SessionMap,
        r: &mut Runtime,
        open: Opening,
        row: SessionRecord,
    ) {
        let OpenPurpose::Recovery { key, token } = open.purpose else {
            return;
        };
        if row.key != key
            || row.lifecycle != SessionLifecycle::Open
            || r.keys.get(&key.client) != Some(&key)
        {
            self.fail_runtime(
                JournalError::Corrupt,
                "recovery attachment changed durable SessionKey",
            );
            r.cancel_pending();
            return;
        }
        // Even an obsolete attachment can have committed its address. Preserve
        // that union before the replacement update or any fencing attempt.
        let durable_open = (row.key, row.identity.open_id);
        if let Some(s) = sessions.get_mut(key.client) {
            s.durable_open = Some(durable_open);
        }
        r.rows.insert(key, row.clone());
        let Some(member) = r.recovery.members.get_mut(&key) else {
            return;
        };
        if member.token != Some(token) || member.disconnected || member.evicting {
            return;
        }
        if open
            .link
            .authenticated_identity()
            .is_none_or(|(name, context)| {
                name != row.identity.authenticated_entity
                    || context != row.identity.authorization_context
            })
            || self.is_blocklisted(&open.link.peer_addr())
            || !self.features_allowed(row.identity.client_features)
        {
            member.evicting = true;
            member.attachment = None;
            Self::recovery_terminal(r, sessions, key.client, open.link, MClientSession::close());
            return;
        }
        if let Some(generation) = sessions.attach_recovery(
            key.client,
            open.generation,
            open.link,
            open.msg,
            open.cap_auths,
        ) {
            if let Some(s) = sessions.get_mut(key.client) {
                s.durable_open = Some(durable_open);
            }
            r.attached.insert(key.client, generation);
            member.attached = true;
        } else {
            self.fail_runtime(
                JournalError::Corrupt,
                "recovery attachment runtime generation changed",
            );
            r.cancel_pending();
        }
    }
    pub fn recovery_connection_gone(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        connection: u64,
    ) {
        let Some(r) = &mut self.runtime else {
            return;
        };
        let Some(key) = r.keys.get(&client).copied() else {
            return;
        };
        let Some(member) = r.recovery.members.get_mut(&key) else {
            return;
        };
        if member.token.is_none_or(|t| t.connection != connection) || member.disconnected {
            return;
        }
        member.disconnected = true;
        member.attachment = None;
        member.attached = false;
        if !member.complete && !member.evicting {
            if self.engine.reset_reconnect(key).is_err() {
                self.fatal_error = Some("incomplete reconnect reset failed".into());
                return;
            }
            r.recovery.reset_partial(key);
            sessions.remove(client);
            r.attached.remove(&client);
        }
        // A final reconstruction survives transport loss. Its replacement can
        // attach and retain sequence/rights, but cannot install a second stream.
    }
    pub(super) fn drive_recovery(&mut self, sessions: &mut SessionMap, budget: usize) {
        self.expire_reconnect(sessions, Instant::now());
        let Some(mut r) = self.runtime.take() else {
            return;
        };
        if !r.recovery.nonempty || !r.initialized || r.execution_paused {
            self.runtime = Some(r);
            return;
        }
        let result = self.recovery_turn(&mut r, sessions, budget);
        if let Err(reason) = result {
            self.fail_runtime(JournalError::Corrupt, reason);
            r.cancel_pending();
        }
        self.runtime = Some(r);
    }
    fn recovery_turn(
        &mut self,
        r: &mut Runtime,
        sessions: &mut SessionMap,
        budget: usize,
    ) -> Result<(), &'static str> {
        // A staged chunk can outlive the map that authorized its attachment.
        // Recheck before any reconstruction or early-control activity/output.
        let denied: Vec<_> = r
            .recovery
            .members
            .iter()
            .filter_map(|(key, member)| {
                if !member.attached || member.evicting {
                    return None;
                }
                let allowed = r.rows.get(key).is_some_and(|row| {
                    row.lifecycle == SessionLifecycle::Open
                        && self.features_allowed(row.identity.client_features)
                        && sessions.get(key.client).is_some_and(|s| {
                            r.attached.get(&key.client) == Some(&s.generation)
                                && !self.is_blocklisted(&s.addr)
                                && s.addr == s.link.peer_addr()
                                && row.identity.peer_addresses.0.contains(&s.addr)
                                && s.link.authenticated_identity().is_some_and(
                                    |(entity, context)| {
                                        entity == row.identity.authenticated_entity
                                            && context == row.identity.authorization_context
                                    },
                                )
                        })
                });
                (!allowed).then_some(*key)
            })
            .collect();
        for key in denied {
            r.freeze_recovery_client(key.client);
            if let Some(member) = r.recovery.members.get_mut(&key) {
                member.close_reply = true;
            }
            if let Some(s) = sessions.get_mut(key.client) {
                s.state = SessionState::Fencing;
            }
        }
        for _ in 0..budget {
            let ready = r
                .recovery
                .members
                .iter()
                .find(|(key, m)| {
                    !m.evicting && m.attachment.is_some() && !r.opening.contains_key(&key.client)
                })
                .map(|(k, _)| *k);
            if let Some(key) = ready {
                let m = r
                    .recovery
                    .members
                    .get_mut(&key)
                    .ok_or("missing attachment member")?;
                let attachment = m.attachment.take().ok_or("missing attachment")?;
                let row = r.rows.get(&key).ok_or("missing rank row")?;
                let Some((entity, context)) = attachment.link.authenticated_identity() else {
                    return Err("attachment credentials disappeared");
                };
                if entity != row.identity.authenticated_entity {
                    return Err("attachment authenticated name changed");
                }
                let mut identity = row.identity.clone();
                if !identity
                    .peer_addresses
                    .0
                    .contains(&attachment.link.peer_addr())
                {
                    identity.peer_addresses.0.push(attachment.link.peer_addr());
                }
                if identity.peer_addresses.0.len() > MAX_ADDRESSES {
                    return Err("recovery peer address union exceeds 64");
                }
                r.next_operation += 1;
                identity.open_id = persistence::operation_id(r.owner.gid, r.next_operation);
                identity.authorization_context = context;
                identity.client_features = attachment.features;
                let mut msg = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
                msg.supported_features = FeatureBitset::from_word(identity.client_features);
                for (k, v) in &identity.metadata {
                    msg.metadata.insert(
                        String::from_utf8(k.to_vec())
                            .map_err(|_| "invalid durable metadata key")?,
                        String::from_utf8(v.to_vec())
                            .map_err(|_| "invalid durable metadata value")?,
                    );
                }
                r.opening_bytes += attachment.bytes;
                r.opening.insert(
                    key.client,
                    Opening {
                        link: attachment.link,
                        msg,
                        cap_auths: attachment.auths,
                        bytes: attachment.bytes,
                        generation: sessions.get(key.client).map(|s| s.generation),
                        new_session: false,
                        purpose: OpenPurpose::Recovery {
                            key,
                            token: attachment.token,
                        },
                    },
                );
                r.command(CommandKind::Open(identity), Pending::Open(key.client));
                continue;
            }
            if !r.recovery.confirmed
                || !r.recovery.collection_allowed
                || self.engine.journal().busy()
                || matches!(
                    r.state,
                    StepState::NeedsAdmission | StepState::WaitingCommit(_)
                )
            {
                break;
            }
            let eligible = |h: &Held| {
                h.key
                    .and_then(|k| r.recovery.members.get(&k))
                    .is_some_and(|m| {
                        m.attached && !m.evicting && !m.disconnected && m.token == Some(h.token)
                    })
            };
            if let Some(index) = r.recovery.chunks.iter().position(eligible) {
                let held = r
                    .recovery
                    .chunks
                    .remove(index)
                    .ok_or("missing reconnect input")?;
                r.recovery.input_bytes -= held.bytes;
                self.apply_reconnect(r, sessions, held)?;
                continue;
            }
            // Renewal is wire-only. Dirty controls and retries remain protected
            // and charged until the separately accepted D5/E2 drain is installed.
            let activity = r
                .recovery
                .early
                .iter()
                .position(|h| eligible(h) && !h.activity_noted);
            if let Some(index) = activity {
                let held = &mut r.recovery.early[index];
                held.activity_noted = true;
                let valid = match &held.msg {
                    ClientMessage::Request(request) => request_from_proto(
                        request,
                        now_ts(),
                        sessions
                            .get(held.key.map_or(0, |k| k.client))
                            .map_or(0, |s| s.link.features()),
                    )
                    .is_ok(),
                    ClientMessage::Caps(cap) => {
                        if !matches!(
                            cap.op,
                            CapOp::UPDATE
                                | CapOp::FLUSH
                                | CapOp::DROP
                                | CapOp::RELEASE
                                | CapOp::RENEW
                                | CapOp::FLUSHSNAP
                        ) || cap_update_from_proto(cap).is_err()
                        {
                            return Err("invalid early recovery cap update");
                        }
                        self.raise_epoch_barrier(cap.osd_epoch_barrier);
                        true
                    }
                    ClientMessage::CapRelease(release) => {
                        self.raise_epoch_barrier(release.osd_epoch_barrier);
                        true
                    }
                    ClientMessage::Session(msg) => msg.op == SessionOp::REQUEST_RENEWCAPS,
                    _ => false,
                };
                let (key, token, received) = (held.key, held.token, held.received);
                if let ClientMessage::Session(msg) = &held.msg
                    && msg.op == SessionOp::REQUEST_RENEWCAPS
                {
                    if let Some(s) = key.and_then(|k| sessions.get_mut(k.client))
                        && !s.send(&MClientSession::renewcaps(msg.seq))
                    {
                        return Err("recovery renewal output failed");
                    }
                    let held = r.recovery.early.remove(index).ok_or("missing renewal")?;
                    r.recovery.input_bytes -= held.bytes;
                }
                if valid && let Some(key) = key {
                    Self::note_recovery_activity(r, key, token, received);
                }
                continue;
            }
            break;
        }
        Ok(())
    }
    fn note_recovery_activity(r: &mut Runtime, key: SessionKey, token: Token, received: Instant) {
        if r.gather.contains(&key)
            && r.recovery
                .members
                .get(&key)
                .is_some_and(|m| m.token == Some(token) && m.attached && !m.evicting)
            && let Some(w) = &mut r.recovery.window
            && !w.expired
        {
            w.last = w.last.max(w.start).max(received);
        }
    }
    fn apply_reconnect(
        &mut self,
        r: &mut Runtime,
        sessions: &mut SessionMap,
        held: Held,
    ) -> Result<(), &'static str> {
        let key = held.key.ok_or("reconnect missing key")?;
        let ClientMessage::Reconnect(msg) = held.msg else {
            return Err("wrong reconnect input");
        };
        let member = r
            .recovery
            .members
            .get(&key)
            .ok_or("unknown reconnect member")?;
        if member.complete {
            if member.final_token != Some(held.token) {
                let m = r
                    .recovery
                    .members
                    .get_mut(&key)
                    .ok_or("missing complete member")?;
                m.evicting = true;
                m.close_reply = true;
                if let Some(s) = sessions.get_mut(key.client) {
                    s.state = SessionState::Fencing;
                }
            }
            return Ok(());
        }
        let mut caps = Vec::new();
        let mut applied = member.caps.clone();
        for (&ino, cap) in &msg.caps {
            let locks = cap
                .locks()
                .map_err(|_| "malformed reconnect lock blob; no chunk installed")?;
            let translate = |l: proto::FileLock| engine::filelock::FileLock {
                start: l.start,
                length: l.length,
                client: key.client,
                owner: l.owner,
                pid: l.pid,
                ltype: l.lock_type,
            };
            let count = locks.fcntl.len() + locks.flock.len();
            applied.insert(
                ino,
                AppliedCap {
                    realm: cap.snaprealm,
                    locks: count,
                    bytes: 256 + count * 64,
                },
            );
            caps.push(engine::ReconnectCap {
                ino,
                cap_id: cap.cap_id,
                wanted: cap.wanted,
                issued: cap.issued,
                realm: cap.snaprealm,
                snap_follows: cap.snap_follows,
                fcntl_locks: locks.fcntl.into_iter().map(translate).collect(),
                flock_locks: locks.flock.into_iter().map(translate).collect(),
            });
        }
        let mut realms = member.realms.clone();
        let mut translated = Vec::new();
        for realm in &msg.realms {
            let realm = engine::ReconnectRealm {
                ino: realm.ino,
                seq: realm.seq,
                parent: realm.parent,
            };
            realms.insert(realm.ino, realm);
            translated.push(realm);
        }
        let features = sessions
            .get(key.client)
            .ok_or("reconnect without silent funnel")?
            .features
            .clone();
        let old = member.bytes();
        let new_bytes = 512
            + member.binding.as_ref().map_or(0, |(_, ctx, _)| ctx.len())
            + applied.values().map(|c| c.bytes).sum::<usize>()
            + realms.len() * 96;
        let other_caps: usize = r
            .recovery
            .members
            .iter()
            .filter(|(k, _)| **k != key)
            .map(|(_, m)| m.caps.len())
            .sum();
        let other_locks: usize = r
            .recovery
            .members
            .iter()
            .filter(|(k, _)| **k != key)
            .flat_map(|(_, m)| m.caps.values())
            .map(|c| c.locks)
            .sum();
        let other_realms: usize = r
            .recovery
            .members
            .iter()
            .filter(|(k, _)| **k != key)
            .map(|(_, m)| m.realms.len())
            .sum();
        if other_caps + applied.len() > MAX_CAPS
            || other_locks + applied.values().map(|c| c.locks).sum::<usize>() > MAX_LOCKS
            || other_realms + realms.len() > MAX_REALMS
            || r.recovery.bytes() - old + new_bytes + r.opening_bytes > MAX_RECOVERY_BYTES
        {
            return Err("applied reconnect aggregate budget exhausted");
        }
        let correction_budget =
            MAX_RECOVERY_BYTES - (r.recovery.bytes() - old + new_bytes + r.opening_bytes);
        let (corrections, correction_bytes) = if !msg.more {
            self.realm_corrections(&applied, &realms, &features, correction_budget)?
        } else {
            (Vec::new(), 0)
        };
        // All lock blobs and retained sizes were validated before this single
        // atomic engine call. Path hints never create or substitute metadata.
        let progress = self
            .engine
            .reconnect_chunk(key, &caps, &translated, msg.more)
            .map_err(|_| "invalid reconnect chunk; no chunk installed")?;
        // The first valid final is still activity of a missing member. Record
        // its receive time before removing that key. Duplicates returned above.
        Self::note_recovery_activity(r, key, held.token, held.received);
        let member = r
            .recovery
            .members
            .get_mut(&key)
            .ok_or("missing reconnect member")?;
        member.caps = applied;
        member.realms = realms;
        member.correction_bytes = correction_bytes;
        if progress.complete {
            member.complete = true;
            member.final_token = Some(held.token);
            r.gather.remove(&key);
            if let Some(s) = sessions.get_mut(key.client) {
                s.last_renew = Instant::now();
                for correction in corrections {
                    if !s.send(&correction) {
                        return Err("recovery correction output failed");
                    }
                }
            }
        }
        Ok(())
    }
    fn realm_corrections(
        &self,
        caps: &BTreeMap<u64, AppliedCap>,
        realms: &BTreeMap<u64, engine::ReconnectRealm>,
        features: &FeatureBitset,
        budget: usize,
    ) -> Result<(Vec<proto::MClientSnap>, usize), &'static str> {
        let format = SnapFormat::for_client(features);
        let mut splits: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for (ino, cap) in caps {
            let trace = self.engine.snap_trace(*ino);
            let realm = trace.first().ok_or("missing authoritative cap realm")?.ino;
            if realm != cap.realm {
                splits.entry(realm).or_default().push(*ino);
            }
        }
        let mut output = Vec::new();
        let mut bytes = 0;
        let mut push = |message: proto::MClientSnap| -> Result<(), &'static str> {
            let raw = message.to_raw(0);
            let charge = (raw.front.len() + raw.data.len()) * 4 + 256;
            if charge > budget.saturating_sub(bytes) {
                return Err("recovery realm correction byte budget exhausted");
            }
            bytes += charge;
            output.push(message);
            Ok(())
        };
        for (realm, inos) in splits {
            let children: Vec<_> = self
                .engine
                .inode_numbers()
                .into_iter()
                .filter(|ino| {
                    self.engine
                        .snap_realm_info(*ino)
                        .is_some_and(|r| r.parent == realm)
                })
                .take(MAX_REALMS + 1)
                .collect();
            if children.len() > MAX_REALMS {
                return Err("realm correction child budget exhausted");
            }
            let trace = SnapTrace::new(
                self.engine
                    .snap_trace(realm)
                    .iter()
                    .map(snap_realm)
                    .collect(),
            )
            .encode(format);
            push(proto::MClientSnap::split(realm, inos, children, trace))?;
        }
        for realm in realms.values() {
            let info = self
                .engine
                .snap_realm_info(realm.ino)
                .ok_or("missing authoritative reported realm")?;
            if info.seq != realm.seq || info.parent != realm.parent {
                let trace = SnapTrace::new(
                    self.engine
                        .snap_trace(realm.ino)
                        .iter()
                        .map(snap_realm)
                        .collect(),
                )
                .encode(format);
                push(proto::MClientSnap::update(realm.ino, trace))?;
            }
        }
        Ok((output, bytes))
    }
    pub fn recovery_fences(&mut self, sessions: &mut SessionMap) -> Vec<RecoveryFence> {
        let Some(r) = &mut self.runtime else {
            return Vec::new();
        };
        let mut targets = r.reclaim_fences(sessions);
        for (key, member) in &r.recovery.members {
            if !member.evicting
                || r.opening.contains_key(&key.client)
                || r.closing.contains_key(key)
            {
                continue;
            }
            let Some(row) = r.rows.get(key) else {
                continue;
            };
            if let Some(s) = sessions.get_mut(key.client) {
                s.state = SessionState::Fencing;
            }
            targets.push(RecoveryFence {
                owner: r.owner,
                key: *key,
                attachment: sessions
                    .get(key.client)
                    .map(|s| (s.link.connection_id(), s.generation)),
                addresses: row.identity.peer_addresses.clone(),
            });
        }
        targets
    }
    pub fn matches_recovery_fence(&self, sessions: &SessionMap, target: &RecoveryFence) -> bool {
        self.runtime.as_ref().is_some_and(|r| {
            r.matches_reclaim_fence(sessions, target)
                || (r.owner == target.owner
                    && r.keys.get(&target.key.client) == Some(&target.key)
                    && !r.opening.contains_key(&target.key.client)
                    && !r.closing.contains_key(&target.key)
                    && r.rows.get(&target.key).is_some_and(|row| {
                        row.lifecycle == SessionLifecycle::Open
                            && row.identity.peer_addresses == target.addresses
                    })
                    && r.recovery
                        .members
                        .get(&target.key)
                        .is_some_and(|m| m.evicting)
                    && sessions
                        .get(target.key.client)
                        .map(|s| (s.link.connection_id(), s.generation))
                        == target.attachment)
        })
    }
    pub fn finish_recovery_eviction(
        &mut self,
        sessions: &mut SessionMap,
        target: &RecoveryFence,
        epoch: u32,
    ) -> bool {
        if epoch == 0
            || self.fatal_error().is_some()
            || !self.matches_recovery_fence(sessions, target)
        {
            return false;
        }
        if self
            .runtime
            .as_ref()
            .is_some_and(|r| r.matches_reclaim_fence(sessions, target))
        {
            return self.finish_reclaim_fence(sessions, target, epoch);
        }
        self.raise_epoch_barrier(epoch);
        if let Some(r) = &mut self.runtime {
            let reply = r
                .recovery
                .members
                .get(&target.key)
                .is_some_and(|m| m.close_reply);
            r.closing.insert(
                target.key,
                Closing {
                    key: target.key,
                    generation: target.attachment.map_or(0, |(_, g)| g),
                    kill: true,
                    reply,
                    request_seq: None,
                    stage: CloseStage::Drain,
                },
            );
            r.blocked = false;
            r.busy_controls = 0;
        }
        self.pump(sessions, 32);
        true
    }
}
