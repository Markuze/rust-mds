//! ACTIVE timers and RESET reclaim. Wire flags/errors follow C++
//! src/mds/Server.cc::{handle_client_reclaim,reclaim_session,find_idle_sessions}.
//! Durable close and strict decimal timeouts follow recovery-contract.md section 4.

use super::*;

const MAX_RECLAIM_WAITERS: usize = 128;

#[derive(Clone, PartialEq, Eq)]
struct Waiter {
    key: SessionKey,
    generation: u64,
    connection: u64,
    open_id: [u8; 16],
}
struct Target {
    fence: Option<RecoveryFence>,
    epoch: Option<u32>,
    waiters: Vec<Waiter>,
}
#[derive(Default)]
pub(super) struct D6 {
    targets: BTreeMap<SessionKey, Target>,
}

fn uuid(row: &SessionRecord) -> Option<&[u8]> {
    row.identity
        .metadata
        .get(b"uuid".as_slice())
        .map(Bytes::as_ref)
}

impl Runtime {
    pub(super) fn uuid_conflict(&self, client: u64, requested: &str) -> bool {
        self.rows.values().any(|row| {
            row.key.client != client
                && row.lifecycle != SessionLifecycle::Closed
                && uuid(row) == Some(requested.as_bytes())
        }) || self.opening.iter().any(|(other, open)| {
            *other != client
                && open
                    .msg
                    .metadata
                    .get("uuid")
                    .is_some_and(|value| value == requested)
        })
    }
    pub(super) fn reclaiming(&self, client: u64) -> bool {
        self.d6.targets.keys().any(|key| key.client == client)
    }
    pub(super) fn reclaim_fences(&mut self, sessions: &SessionMap) -> Vec<RecoveryFence> {
        let mut targets = Vec::new();
        for (key, pending) in &mut self.d6.targets {
            if pending.epoch.is_some() || self.opening.contains_key(&key.client) {
                continue;
            }
            let Some(row) = self.rows.get(key) else {
                continue;
            };
            let target = pending.fence.get_or_insert_with(|| RecoveryFence {
                owner: self.owner,
                key: *key,
                attachment: sessions
                    .get(key.client)
                    .map(|s| (s.link.connection_id(), s.generation)),
                addresses: row.identity.peer_addresses.clone(),
            });
            targets.push(target.clone());
        }
        targets
    }
    pub(super) fn matches_reclaim_fence(
        &self,
        sessions: &SessionMap,
        target: &RecoveryFence,
    ) -> bool {
        self.owner == target.owner
            && self.d6.targets.get(&target.key).is_some_and(|pending| {
                pending.epoch.is_none() && pending.fence.as_ref() == Some(target)
            })
            && !self.opening.contains_key(&target.key.client)
            && self.rows.get(&target.key).is_some_and(|row| {
                row.identity.peer_addresses == target.addresses
                    && (row.lifecycle == SessionLifecycle::Closed
                        || (self.keys.get(&target.key.client) == Some(&target.key)
                            && sessions
                                .get(target.key.client)
                                .is_some_and(|s| s.state == SessionState::Fencing)
                            && sessions
                                .get(target.key.client)
                                .map(|s| (s.link.connection_id(), s.generation))
                                == target.attachment))
            })
    }
    pub(super) fn finish_reclaims(
        &mut self,
        sessions: &mut SessionMap,
        required_features: &[u8],
        objecter: Option<&Objecter>,
    ) {
        let done: Vec<_> = self
            .d6
            .targets
            .iter()
            .filter_map(|(key, pending)| {
                (pending.epoch.is_some()
                    && self
                        .rows
                        .get(key)
                        .is_some_and(|r| r.lifecycle == SessionLifecycle::Closed))
                .then_some(*key)
            })
            .collect();
        for key in done {
            let Some(pending) = self.d6.targets.remove(&key) else {
                continue;
            };
            let (Some(fence), Some(epoch)) = (pending.fence, pending.epoch) else {
                continue;
            };
            for waiter in pending.waiters {
                if self.keys.get(&waiter.key.client) != Some(&waiter.key)
                    || self.attached.get(&waiter.key.client) != Some(&waiter.generation)
                    || !self.rows.get(&waiter.key).is_some_and(|row| {
                        row.lifecycle == SessionLifecycle::Open
                            && row.identity.open_id == waiter.open_id
                    })
                {
                    continue;
                }
                if let Some(s) = sessions.get_mut(waiter.key.client)
                    && s.is_open()
                    && s.generation == waiter.generation
                    && s.link.connection_id() == waiter.connection
                    && s.addr == s.link.peer_addr()
                    && !objecter.is_some_and(|objecter| objecter.osdmap().is_blocklisted(&s.addr))
                    && self.rows.get(&waiter.key).is_some_and(|row| {
                        row.identity.peer_addresses.0.contains(&s.addr)
                            && s.link
                                .authenticated_identity()
                                .is_some_and(|(name, context)| {
                                    name == row.identity.authenticated_entity
                                        && context == row.identity.authorization_context
                                })
                            && required_features.iter().enumerate().all(|(i, required)| {
                                let offered = if i < 8 {
                                    (row.identity.client_features >> (i * 8)) as u8
                                } else {
                                    0
                                };
                                required & !offered == 0
                            })
                    })
                    && !s.send(&MClientReclaimReply {
                        result: 0,
                        epoch,
                        addrs: fence.addresses.clone(),
                    })
                {
                    return;
                }
            }
        }
    }
}

impl Server<DaemonJournal> {
    /// Validate activity before a bounded raw queue can delay dispatch. This
    /// changes only the monotonic clock, not admission, state, output or rights.
    pub fn note_held_activity(
        &self,
        sessions: &mut SessionMap,
        client: u64,
        link: &dyn ClientLink,
        raw: &denc::RawMessage,
        received: Instant,
    ) {
        if self.fatal_error().is_some()
            || sessions.output_failed()
            || !self.recovered()
            || self.is_blocklisted(&link.peer_addr())
            || !self.is_attached(sessions, client, link)
            || !sessions.get(client).is_some_and(|s| s.is_open())
        {
            return;
        }
        let valid = match ClientMessage::from_raw(raw.clone()) {
            Ok(Some(ClientMessage::Request(request))) => {
                request_from_proto(&request, now_ts(), link.features())
                    .is_ok_and(|request| !matches!(request.op, Op::Other { .. }))
            }
            Ok(Some(ClientMessage::Caps(cap))) => {
                matches!(
                    cap.op,
                    CapOp::UPDATE
                        | CapOp::FLUSH
                        | CapOp::DROP
                        | CapOp::RELEASE
                        | CapOp::RENEW
                        | CapOp::FLUSHSNAP
                ) && cap_update_from_proto(&cap).is_ok()
            }
            Ok(Some(ClientMessage::CapRelease(_) | ClientMessage::Metrics(_))) => true,
            Ok(Some(ClientMessage::Session(msg))) => matches!(
                msg.op,
                SessionOp::REQUEST_RENEWCAPS
                    | SessionOp::REQUEST_FLUSH_MDLOG
                    | SessionOp::FLUSHMSG_ACK
            ),
            Ok(Some(ClientMessage::Reclaim(msg))) => {
                msg.flags == proto::reclaim::RECLAIM_FLAG_FINISH
                    || (msg.flags == proto::reclaim::CEPH_RECLAIM_RESET
                        && !msg.uuid.is_empty()
                        && self.runtime.as_ref().is_some_and(|r| {
                            let requester = r.keys.get(&client).and_then(|key| r.rows.get(key));
                            r.rows
                                .values()
                                .find(|row| {
                                    row.lifecycle != SessionLifecycle::Closed
                                        && uuid(row) == Some(msg.uuid.as_bytes())
                                })
                                .is_none_or(|target| {
                                    requester.is_some_and(|row| {
                                        row.identity.authenticated_entity
                                            == target.identity.authenticated_entity
                                    })
                                })
                        }))
            }
            _ => false,
        };
        if valid && let Some(s) = sessions.get_mut(client) {
            s.last_renew = s.last_renew.max(received);
        }
    }

    pub(super) fn reclaim(
        &mut self,
        sessions: &mut SessionMap,
        key: SessionKey,
        msg: proto::MClientReclaim,
        received: Instant,
    ) {
        let Some(r) = &mut self.runtime else {
            return;
        };
        let Some(s) = sessions
            .get(key.client)
            .filter(|s| s.is_open() && r.attached.get(&key.client) == Some(&s.generation))
        else {
            return;
        };
        let Some(row) = r
            .rows
            .get(&key)
            .filter(|row| row.lifecycle == SessionLifecycle::Open)
        else {
            return;
        };
        let requester = Waiter {
            key,
            generation: s.generation,
            connection: s.link.connection_id(),
            open_id: row.identity.open_id,
        };
        let mut result = 0;
        let mut target = None;
        if msg.flags == proto::reclaim::RECLAIM_FLAG_FINISH {
            if let Some(s) = sessions.get_mut(key.client) {
                s.last_renew = s.last_renew.max(received);
            }
            return;
        } else if msg.flags != proto::reclaim::CEPH_RECLAIM_RESET || msg.uuid.is_empty() {
            result = -22;
        } else if let Some(old) = r.rows.values().find(|old| {
            old.lifecycle != SessionLifecycle::Closed && uuid(old) == Some(msg.uuid.as_bytes())
        }) {
            if old.identity.authenticated_entity != row.identity.authenticated_entity {
                result = -1;
            } else {
                target = Some(old.key);
            }
        }
        if result == 0
            && let Some(s) = sessions.get_mut(key.client)
        {
            s.last_renew = s.last_renew.max(received);
        }
        let Some(target) = target else {
            if let Some(s) = sessions.get_mut(key.client) {
                s.send(&MClientReclaimReply {
                    result,
                    ..Default::default()
                });
            }
            self.check_output_failure(sessions);
            return;
        };
        if r.d6
            .targets
            .get(&target)
            .is_some_and(|pending| pending.waiters.contains(&requester))
        {
            return;
        }
        if r.d6
            .targets
            .values()
            .map(|pending| pending.waiters.len())
            .sum::<usize>()
            >= MAX_RECLAIM_WAITERS
        {
            self.fail_runtime(
                JournalError::Capacity,
                "RESET reclaim waiter capacity exceeded (128)",
            );
            return;
        }
        r.d6.targets
            .entry(target)
            .or_insert_with(|| Target {
                fence: None,
                epoch: None,
                waiters: Vec::new(),
            })
            .waiters
            .push(requester);
        r.cancel_requests(target);
        if let Some(close) = r.closing.get_mut(&target)
            && matches!(close.stage, CloseStage::Drain | CloseStage::WaitSeq)
        {
            close.stage = CloseStage::WaitFence;
        }
        if let Some(s) = sessions.get_mut(target.client) {
            s.state = SessionState::Fencing;
        }
        // The existing full-union tagged fence consumer serializes cleanup.
    }

    pub(super) fn finish_reclaim_fence(
        &mut self,
        sessions: &mut SessionMap,
        target: &RecoveryFence,
        epoch: u32,
    ) -> bool {
        if !self
            .runtime
            .as_ref()
            .is_some_and(|r| r.matches_reclaim_fence(sessions, target))
        {
            return false;
        }
        self.raise_epoch_barrier(epoch);
        let Some(r) = &mut self.runtime else {
            return false;
        };
        if let Some(pending) = r.d6.targets.get_mut(&target.key) {
            pending.epoch = Some(epoch);
        }
        if r.rows
            .get(&target.key)
            .is_some_and(|row| row.lifecycle != SessionLifecycle::Closed)
        {
            r.closing
                .entry(target.key)
                .and_modify(|close| {
                    close.kill = true;
                    close.request_seq = None;
                    if matches!(
                        close.stage,
                        CloseStage::Drain | CloseStage::WaitSeq | CloseStage::WaitFence
                    ) {
                        close.stage = CloseStage::Drain;
                    }
                })
                .or_insert(Closing {
                    key: target.key,
                    generation: target.attachment.map_or(0, |(_, g)| g),
                    kill: true,
                    reply: false,
                    request_seq: None,
                    stage: CloseStage::Drain,
                });
        }
        r.blocked = false;
        r.busy_controls = 0;
        r.finish_reclaims(
            sessions,
            &self.required_client_features,
            self.objecter.as_ref(),
        );
        self.pump(sessions, 32);
        true
    }

    pub(super) fn active_timers(&mut self, sessions: &mut SessionMap, now: Instant) {
        if self.fatal_error().is_some() || sessions.output_failed() {
            return;
        }
        let mut clients = sessions.clients();
        clients.sort_unstable();
        for client in clients {
            let Some(r) = &self.runtime else {
                return;
            };
            let Some(s) = sessions.get(client).filter(|s| s.is_open()) else {
                continue;
            };
            let Some(row) = r
                .keys
                .get(&client)
                .and_then(|key| r.rows.get(key))
                .filter(|row| row.lifecycle == SessionLifecycle::Open)
            else {
                continue;
            };
            let idle = now.saturating_duration_since(s.last_renew);
            if idle < self.session_timeout {
                continue;
            }
            if idle >= self.session_autoclose {
                // Autoclose precedes all timeout metadata interpretation.
                self.request_eviction(sessions, client);
                self.check_output_failure(sessions);
                if self.fatal_error().is_some() {
                    return;
                }
                continue;
            }
            let timeout = row.identity.metadata.get(b"timeout".as_slice());
            let override_seconds = timeout.and_then(|value| {
                let value = std::str::from_utf8(value).ok().map(str::trim);
                let parsed = value.filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
                    .and_then(|v| v.parse::<u32>().ok());
                if parsed.is_none() { warn!(client, "invalid decimal session timeout; retaining timeout-session policy and stored text"); }
                parsed.filter(|seconds| *seconds != 0)
            });
            if override_seconds.is_some_and(|p| idle >= Duration::from_secs(u64::from(p))) {
                self.request_eviction(sessions, client);
            } else if timeout.is_none()
                && s.state == SessionState::Open
                && let Some(s) = sessions.get_mut(client)
            {
                // Do not call the compatibility stale API: writer rights and
                // locks remain held until acknowledged release or fencing.
                s.state = SessionState::Stale;
                s.send(&MClientSession::stale());
            }
            self.check_output_failure(sessions);
            if self.fatal_error().is_some() {
                return;
            }
        }
    }
}
