//! Rank-session authority and lifecycle. Every mutation compares this object's
//! own authority/revision. No WAL guard grants permission to change this object.

use crate::{
    backend::{Backend, checked},
    stable::{Authority, FencedWriter, RankOwner, StoreError, StoreIdentity, WriterIdentity},
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use denc::{Decode, Encode, EntityAddrVec};
use engine::durability::SessionKey;
use osdc::{ObjectOperation, ops::cmp_op};
use std::collections::{BTreeMap, BTreeSet};

const IDENTITY: &str = "rmds.identity";
const AUTHORITY: &str = "rmds.authority";
const COUNTER: &str = "rmds.next_incarnation";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSessionRecord {
    pub client: u64,
    /// Stable logical operation id. Retain it across an uncertain OPEN.
    pub open_id: [u8; 16],
    pub authenticated_entity: String,
    pub authorization_context: Bytes,
    pub peer_addresses: EntityAddrVec,
    pub client_features: u64,
    pub server_features: u64,
    /// Includes root, uuid and timeout. This is durable metadata, not a clock.
    pub metadata: BTreeMap<Bytes, Bytes>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLifecycle {
    Open,
    Closing,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub key: SessionKey,
    pub identity: NewSessionRecord,
    pub lifecycle: SessionLifecycle,
    pub cleanup_seq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionsInspection {
    pub owner: RankOwner,
    pub writer: WriterIdentity,
    pub predecessor: Option<FencedWriter>,
    pub revision: u64,
    pub next_incarnation: u64,
}

#[derive(Debug, Clone)]
pub struct AcquiredSessions {
    pub control: SessionsInspection,
}
#[derive(Debug, Clone)]
pub struct RankSessionState {
    pub control: SessionsInspection,
    pub records: Vec<SessionRecord>,
}

#[derive(Clone, PartialEq, Eq)]
enum Action {
    Open(NewSessionRecord),
    Closing(SessionKey),
    Closed(SessionKey, u64),
}
#[derive(Clone)]
struct Pending {
    action: Action,
    before: SessionsInspection,
    after: SessionsInspection,
    row: SessionRecord,
    op: ObjectOperation,
}

/// One task owns one repository. Unknown mutation outcomes block other actions.
pub struct SessionRepository<B> {
    backend: B,
    identity: StoreIdentity,
    rank: u64,
    max_records: usize,
    pending: Option<Pending>,
}

impl<B: Backend> SessionRepository<B> {
    pub fn new(
        backend: B,
        identity: StoreIdentity,
        rank: u64,
        max_records: usize,
    ) -> Result<Self, StoreError> {
        if max_records == 0 {
            return Err(StoreError::Capacity);
        }
        Ok(Self {
            backend,
            identity,
            rank,
            max_records,
            pending: None,
        })
    }
    fn name(&self) -> String {
        format!("rmds.sessions.{}", self.rank)
    }

    pub async fn inspect_sessions(&self) -> Result<SessionsInspection, StoreError> {
        let mut op = ObjectOperation::new();
        op.getxattr(IDENTITY).getxattr(AUTHORITY).getxattr(COUNTER);
        let reply = checked(&self.backend, &self.name(), op).await?;
        if reply.out(0) != self.identity.encode() {
            return Err(StoreError::WrongFormat);
        }
        let authority = Authority::decode(&reply.out(1))?;
        if authority.writer.rank_owner.rank != self.rank {
            return Err(StoreError::Corrupt);
        }
        let next_incarnation = read_counter(&reply.out(2))?;
        if next_incarnation == 0 {
            return Err(StoreError::Corrupt);
        }
        Ok(SessionsInspection {
            owner: authority.writer.rank_owner,
            writer: authority.writer,
            predecessor: authority.predecessor,
            revision: authority.revision,
            next_incarnation,
        })
    }

    pub async fn acquire_sessions(
        &self,
        expected: &SessionsInspection,
        new_owner: RankOwner,
        writer: WriterIdentity,
        predecessor: FencedWriter,
    ) -> Result<AcquiredSessions, StoreError> {
        if self.pending.is_some() {
            return Err(StoreError::UnknownOutcome);
        }
        if new_owner.rank != self.rank
            || writer.rank_owner != new_owner
            || writer.addresses.0.is_empty()
            || expected.writer != predecessor.writer
            || predecessor.verified_osd_epoch == 0
            || new_owner.incarnation <= expected.owner.incarnation
            || new_owner.gid == expected.owner.gid
        {
            return Err(StoreError::Fenced);
        }
        let desired = SessionsInspection {
            owner: new_owner,
            writer,
            predecessor: Some(predecessor),
            revision: expected
                .revision
                .checked_add(1)
                .ok_or(StoreError::Capacity)?,
            next_incarnation: expected.next_incarnation,
        };
        let current = self.inspect_sessions().await?;
        if current == desired {
            return Ok(AcquiredSessions { control: current });
        }
        if current != *expected {
            return Err(StoreError::Fenced);
        }
        let mut op = self.guarded(expected)?;
        op.setxattr(AUTHORITY, authority(&desired).encode()?);
        match checked(&self.backend, &self.name(), op).await {
            Ok(_) | Err(StoreError::UnknownOutcome | StoreError::GuardRejected) => {
                let now = self.inspect_sessions().await?;
                if now == desired {
                    Ok(AcquiredSessions { control: now })
                } else if now == *expected {
                    Err(StoreError::UnknownOutcome)
                } else {
                    Err(StoreError::Fenced)
                }
            }
            Err(e) => Err(e),
        }
    }

    fn guarded(&self, control: &SessionsInspection) -> Result<ObjectOperation, StoreError> {
        if control.owner != control.writer.rank_owner || control.owner.rank != self.rank {
            return Err(StoreError::Fenced);
        }
        let mut op = ObjectOperation::new();
        op.cmpxattr_str(IDENTITY, cmp_op::EQ, &self.identity.encode());
        op.cmpxattr_str(AUTHORITY, cmp_op::EQ, &authority(control).encode()?);
        op.cmpxattr_str(
            COUNTER,
            cmp_op::EQ,
            control.next_incarnation.to_string().as_bytes(),
        );
        Ok(op)
    }

    pub async fn load_sessions(
        &self,
        acquired: &AcquiredSessions,
    ) -> Result<RankSessionState, StoreError> {
        let control = self.inspect_sessions().await?;
        if control.owner != acquired.control.owner || control.writer != acquired.control.writer {
            return Err(StoreError::Fenced);
        }
        let mut records = Vec::new();
        let mut after = Bytes::new();
        let mut live = BTreeSet::new();
        let mut latest = BTreeMap::<u64, SessionKey>::new();
        loop {
            let mut op = self.guarded(&control)?;
            op.omap_get_vals(&after, b"s/", 16);
            let reply = checked(&self.backend, &self.name(), op).await?;
            let (values, more) = decode_map(&reply.out(3), 16, true)?;
            if values.is_empty() && more {
                return Err(StoreError::Corrupt);
            }
            for (key, value) in values {
                if key <= after || !key.starts_with(b"s/") {
                    return Err(StoreError::Corrupt);
                }
                let row = decode_row(&value)?;
                if row_key(row.key) != key || row.key.incarnation >= control.next_incarnation {
                    return Err(StoreError::Corrupt);
                }
                if row.lifecycle != SessionLifecycle::Closed && !live.insert(row.key.client) {
                    return Err(StoreError::Corrupt);
                }
                latest
                    .entry(row.key.client)
                    .and_modify(|key| {
                        if row.key.incarnation > key.incarnation {
                            *key = row.key;
                        }
                    })
                    .or_insert(row.key);
                after = key;
                records.push(row);
                if records.len() > self.max_records {
                    return Err(StoreError::Capacity);
                }
            }
            if !more {
                break;
            }
        }
        let expected: Vec<_> = latest.into_values().collect();
        for group in expected.chunks(16) {
            let keys = group.iter().map(|key| client_key(key.client)).collect();
            let values = self.get_values(&control, keys).await?;
            for key in group {
                if values.get(&client_key(key.client)) != Some(&encode_key(*key)) {
                    return Err(StoreError::Corrupt);
                }
            }
        }
        Ok(RankSessionState { control, records })
    }

    async fn get_values(
        &self,
        control: &SessionsInspection,
        keys: BTreeSet<Bytes>,
    ) -> Result<BTreeMap<Bytes, Bytes>, StoreError> {
        let mut op = self.guarded(control)?;
        op.omap_get_vals_by_keys(&keys);
        let r = checked(&self.backend, &self.name(), op).await?;
        let (values, _) = decode_map(&r.out(3), keys.len(), false)?;
        if values.keys().any(|k| !keys.contains(k)) {
            return Err(StoreError::Corrupt);
        }
        Ok(values)
    }

    async fn get_row(
        &self,
        control: &SessionsInspection,
        key: SessionKey,
    ) -> Result<Option<SessionRecord>, StoreError> {
        let k = row_key(key);
        let row = self
            .get_values(control, [k.clone()].into_iter().collect())
            .await?
            .get(&k)
            .map(decode_row)
            .transpose()?;
        if row.as_ref().is_some_and(|row| row.key != key) {
            return Err(StoreError::Corrupt);
        }
        Ok(row)
    }

    pub async fn open_session_record(
        &mut self,
        acquired: &AcquiredSessions,
        expected_revision: u64,
        record: NewSessionRecord,
    ) -> Result<(SessionKey, u64), StoreError> {
        let action = Action::Open(record.clone());
        if let Some(pending) = &self.pending {
            if pending.action != action || pending.before.revision != expected_revision {
                return Err(StoreError::UnknownOutcome);
            }
            return self.finish_pending(acquired).await;
        }
        let current = self.inspect_sessions().await?;
        if current.owner != acquired.control.owner {
            return Err(StoreError::Fenced);
        }
        let index = client_key(record.client);
        let existing = self
            .get_values(&current, [index.clone()].into_iter().collect())
            .await?;
        let row = if let Some(value) = existing.get(&index) {
            let key = decode_key(value)?;
            let old = self
                .get_row(&current, key)
                .await?
                .ok_or(StoreError::Corrupt)?;
            if old.key.client != record.client {
                return Err(StoreError::Corrupt);
            }
            match old.lifecycle {
                SessionLifecycle::Closing => return Err(StoreError::GuardRejected),
                SessionLifecycle::Open if old.identity == record => {
                    return Ok((key, current.revision));
                }
                SessionLifecycle::Open => SessionRecord {
                    key,
                    identity: record,
                    lifecycle: SessionLifecycle::Open,
                    cleanup_seq: None,
                },
                SessionLifecycle::Closed => SessionRecord {
                    key: SessionKey {
                        client: record.client,
                        incarnation: current.next_incarnation,
                    },
                    identity: record,
                    lifecycle: SessionLifecycle::Open,
                    cleanup_seq: None,
                },
            }
        } else {
            SessionRecord {
                key: SessionKey {
                    client: record.client,
                    incarnation: current.next_incarnation,
                },
                identity: record,
                lifecycle: SessionLifecycle::Open,
                cleanup_seq: None,
            }
        };
        if current.revision != expected_revision {
            return Err(StoreError::GuardRejected);
        }
        let allocated = row.key.incarnation == current.next_incarnation;
        if allocated && current.next_incarnation > self.max_records as u64 {
            return Err(StoreError::Capacity);
        }
        self.stage(action, current, row, allocated)?;
        self.finish_pending(acquired).await
    }

    pub async fn mark_session_closing(
        &mut self,
        acquired: &AcquiredSessions,
        expected_revision: u64,
        key: SessionKey,
    ) -> Result<u64, StoreError> {
        self.lifecycle(acquired, expected_revision, Action::Closing(key))
            .await
    }
    pub async fn mark_session_closed(
        &mut self,
        acquired: &AcquiredSessions,
        expected_revision: u64,
        key: SessionKey,
        cleanup_seq: u64,
    ) -> Result<u64, StoreError> {
        if cleanup_seq == 0 {
            return Err(StoreError::Corrupt);
        }
        self.lifecycle(
            acquired,
            expected_revision,
            Action::Closed(key, cleanup_seq),
        )
        .await
    }

    async fn lifecycle(
        &mut self,
        acquired: &AcquiredSessions,
        expected_revision: u64,
        action: Action,
    ) -> Result<u64, StoreError> {
        if let Some(pending) = &self.pending {
            if pending.action != action || pending.before.revision != expected_revision {
                return Err(StoreError::UnknownOutcome);
            }
            return self
                .finish_pending(acquired)
                .await
                .map(|(_, revision)| revision);
        }
        let current = self.inspect_sessions().await?;
        if current.owner != acquired.control.owner {
            return Err(StoreError::Fenced);
        }
        let (key, target, cleanup_seq) = match action {
            Action::Closing(k) => (k, SessionLifecycle::Closing, None),
            Action::Closed(k, seq) => (k, SessionLifecycle::Closed, Some(seq)),
            _ => return Err(StoreError::Corrupt),
        };
        let mut row = self
            .get_row(&current, key)
            .await?
            .ok_or(StoreError::MissingState)?;
        if row.lifecycle == target && row.cleanup_seq == cleanup_seq {
            return Ok(current.revision);
        }
        if current.revision != expected_revision {
            return Err(StoreError::GuardRejected);
        }
        if !matches!(
            (row.lifecycle, target),
            (SessionLifecycle::Open, SessionLifecycle::Closing)
                | (SessionLifecycle::Closing, SessionLifecycle::Closed)
        ) {
            return Err(StoreError::GuardRejected);
        }
        row.lifecycle = target;
        row.cleanup_seq = cleanup_seq;
        self.stage(action, current, row, false)?;
        self.finish_pending(acquired)
            .await
            .map(|(_, revision)| revision)
    }

    fn stage(
        &mut self,
        action: Action,
        before: SessionsInspection,
        row: SessionRecord,
        allocate: bool,
    ) -> Result<(), StoreError> {
        let mut after = before.clone();
        after.revision = after.revision.checked_add(1).ok_or(StoreError::Capacity)?;
        if allocate {
            after.next_incarnation = after
                .next_incarnation
                .checked_add(1)
                .ok_or(StoreError::Capacity)?;
        }
        let mut op = self.guarded(&before)?;
        let mut values = BTreeMap::new();
        values.insert(row_key(row.key), encode_row(&row)?);
        values.insert(client_key(row.key.client), encode_key(row.key));
        op.omap_set_vals(&values);
        op.setxattr(AUTHORITY, authority(&after).encode()?);
        op.setxattr(COUNTER, Bytes::from(after.next_incarnation.to_string()));
        self.pending = Some(Pending {
            action,
            before,
            after,
            row,
            op,
        });
        Ok(())
    }

    async fn finish_pending(
        &mut self,
        acquired: &AcquiredSessions,
    ) -> Result<(SessionKey, u64), StoreError> {
        let pending = self.pending.clone().ok_or(StoreError::MissingState)?;
        let control = self.inspect_sessions().await?;
        if control.owner != acquired.control.owner || control.owner != pending.before.owner {
            return Err(StoreError::Fenced);
        }
        if control == pending.after {
            if self.get_row(&control, pending.row.key).await? != Some(pending.row.clone()) {
                return Err(StoreError::Corrupt);
            }
            self.pending = None;
            return Ok((pending.row.key, control.revision));
        }
        if control != pending.before {
            return Err(StoreError::Corrupt);
        }
        checked(&self.backend, &self.name(), pending.op).await?;
        self.pending = None;
        Ok((pending.row.key, pending.after.revision))
    }
}

fn authority(c: &SessionsInspection) -> Authority {
    Authority {
        revision: c.revision,
        writer: c.writer.clone(),
        predecessor: c.predecessor.clone(),
    }
}
fn row_key(key: SessionKey) -> Bytes {
    Bytes::from(format!("s/{:016x}/{:016x}", key.client, key.incarnation))
}
fn client_key(client: u64) -> Bytes {
    Bytes::from(format!("c/{client:016x}"))
}
fn encode_key(key: SessionKey) -> Bytes {
    let mut b = BytesMut::new();
    b.put_u64_le(key.client);
    b.put_u64_le(key.incarnation);
    b.freeze()
}
fn decode_key(bytes: &Bytes) -> Result<SessionKey, StoreError> {
    if bytes.len() != 16 {
        return Err(StoreError::Corrupt);
    }
    let mut b = bytes.clone();
    Ok(SessionKey {
        client: b.get_u64_le(),
        incarnation: b.get_u64_le(),
    })
}
fn read_counter(b: &Bytes) -> Result<u64, StoreError> {
    let text = std::str::from_utf8(b).map_err(|_| StoreError::Corrupt)?;
    let n = text.parse::<u64>().map_err(|_| StoreError::Corrupt)?;
    if n.to_string().as_bytes() != b.as_ref() {
        return Err(StoreError::Corrupt);
    }
    Ok(n)
}

fn put_blob(b: &mut BytesMut, bytes: &[u8]) {
    b.put_u32_le(bytes.len() as u32);
    b.extend_from_slice(bytes);
}
fn blob(b: &mut Bytes, max: usize) -> Result<Bytes, StoreError> {
    if b.len() < 4 {
        return Err(StoreError::Corrupt);
    }
    let n = b.get_u32_le() as usize;
    if n > max {
        return Err(StoreError::Capacity);
    }
    if n > b.len() {
        return Err(StoreError::Corrupt);
    }
    Ok(b.split_to(n))
}

fn encode_row(row: &SessionRecord) -> Result<Bytes, StoreError> {
    let r = &row.identity;
    if r.authenticated_entity.len() > 4096
        || r.authorization_context.len() > 65536
        || r.metadata.len() > 128
        || r.peer_addresses.0.len() > 16
        || r.peer_addresses.0.is_empty()
    {
        return Err(StoreError::Capacity);
    }
    let mut b = BytesMut::new();
    let start = denc::encode_start(&mut b, 1, 1);
    b.extend_from_slice(&encode_key(row.key));
    let identity = denc::encode_start(&mut b, 1, 1);
    b.extend_from_slice(&r.open_id);
    put_blob(&mut b, r.authenticated_entity.as_bytes());
    put_blob(&mut b, &r.authorization_context);
    r.peer_addresses.encode(&mut b, denc::features::MSG_ADDR2);
    b.put_u64_le(r.client_features);
    b.put_u64_le(r.server_features);
    b.put_u32_le(r.metadata.len() as u32);
    for (key, value) in &r.metadata {
        if key.len() > 4096 || value.len() > 4096 {
            return Err(StoreError::Capacity);
        }
        put_blob(&mut b, key);
        put_blob(&mut b, value);
    }
    denc::encode_finish(&mut b, identity);
    b.put_u8(match row.lifecycle {
        SessionLifecycle::Open => 0,
        SessionLifecycle::Closing => 1,
        SessionLifecycle::Closed => 2,
    });
    b.put_u64_le(row.cleanup_seq.unwrap_or(0));
    denc::encode_finish(&mut b, start);
    b.put_u32_le(crc32c::crc32c(&b));
    if b.len() > 262144 {
        return Err(StoreError::Capacity);
    }
    Ok(b.freeze())
}

fn frame(b: &mut Bytes) -> Result<Bytes, StoreError> {
    let f = denc::decode_start(b, 1, "rank session").map_err(|_| StoreError::Corrupt)?;
    if f.v != 1 || f.compat != 1 {
        return Err(StoreError::UnsupportedVersion);
    }
    Ok(f.body)
}
fn decode_row(bytes: &Bytes) -> Result<SessionRecord, StoreError> {
    if bytes.len() < 10 || bytes.len() > 262144 {
        return Err(StoreError::Corrupt);
    }
    let mut checksum = bytes.slice(bytes.len() - 4..);
    if crc32c::crc32c(&bytes[..bytes.len() - 4]) != checksum.get_u32_le() {
        return Err(StoreError::Corrupt);
    }
    let mut input = bytes.slice(..bytes.len() - 4);
    let mut b = frame(&mut input)?;
    if !input.is_empty() || b.len() < 16 {
        return Err(StoreError::Corrupt);
    }
    let key = decode_key(&b.split_to(16))?;
    if key.incarnation == 0 {
        return Err(StoreError::Corrupt);
    }
    let mut r = frame(&mut b)?;
    if r.len() < 16 {
        return Err(StoreError::Corrupt);
    }
    let mut open_id = [0; 16];
    r.copy_to_slice(&mut open_id);
    let authenticated_entity =
        String::from_utf8(blob(&mut r, 4096)?.to_vec()).map_err(|_| StoreError::Corrupt)?;
    let authorization_context = blob(&mut r, 65536)?;
    if r.len() < 5 || r[0] != 2 {
        return Err(StoreError::Corrupt);
    }
    let mut count = r.slice(1..5);
    let count = count.get_u32_le();
    if count == 0 || count > 16 {
        return Err(StoreError::Corrupt);
    }
    let peer_addresses = EntityAddrVec::decode(&mut r).map_err(|_| StoreError::Corrupt)?;
    if r.len() < 20 {
        return Err(StoreError::Corrupt);
    }
    let client_features = r.get_u64_le();
    let server_features = r.get_u64_le();
    let n = r.get_u32_le();
    if n > 128 {
        return Err(StoreError::Capacity);
    }
    let mut metadata = BTreeMap::new();
    for _ in 0..n {
        let key = blob(&mut r, 4096)?;
        let value = blob(&mut r, 4096)?;
        if metadata.insert(key, value).is_some() {
            return Err(StoreError::Corrupt);
        }
    }
    if !r.is_empty() || b.len() != 9 {
        return Err(StoreError::Corrupt);
    }
    let lifecycle = match b.get_u8() {
        0 => SessionLifecycle::Open,
        1 => SessionLifecycle::Closing,
        2 => SessionLifecycle::Closed,
        _ => return Err(StoreError::Corrupt),
    };
    let seq = b.get_u64_le();
    if (lifecycle == SessionLifecycle::Closed) != (seq != 0) {
        return Err(StoreError::Corrupt);
    }
    Ok(SessionRecord {
        key,
        identity: NewSessionRecord {
            client: key.client,
            open_id,
            authenticated_entity,
            authorization_context,
            peer_addresses,
            client_features,
            server_features,
            metadata,
        },
        lifecycle,
        cleanup_seq: (seq != 0).then_some(seq),
    })
}

fn decode_map(
    bytes: &Bytes,
    max: usize,
    paged: bool,
) -> Result<(BTreeMap<Bytes, Bytes>, bool), StoreError> {
    let mut b = bytes.clone();
    if b.len() < 4 {
        return Err(StoreError::Corrupt);
    }
    let n = b.get_u32_le() as usize;
    if n > max {
        return Err(StoreError::Capacity);
    }
    let mut values = BTreeMap::new();
    for _ in 0..n {
        let key = blob(&mut b, 35)?;
        let value = blob(&mut b, 262144)?;
        if values.insert(key, value).is_some() {
            return Err(StoreError::Corrupt);
        }
    }
    let more = if paged {
        if b.len() != 1 || b[0] > 1 {
            return Err(StoreError::Corrupt);
        }
        b.get_u8() != 0
    } else {
        false
    };
    if !b.is_empty() {
        return Err(StoreError::Corrupt);
    }
    Ok((values, more))
}
