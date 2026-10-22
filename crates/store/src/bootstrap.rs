//! Preparing/Ready bootstrap with a persisted deterministic plan. The format
//! object has its own authority. Pool listing is native and spans all namespaces.
//! Assignment and predecessor fencing remain daemon policy, not a pool lock.

use crate::{
    backend::{Backend, PoolListing, checked},
    framing::{self, FrameKind},
    record,
    stable::{
        Authority, CheckpointDescriptor, FencedWriter, Limits, OpenedShard, OwnerToken,
        StableStore, StoreError, StoreIdentity, WriterIdentity,
    },
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use denc::Encode;
use engine::{durability::EngineCheckpoint, types::InoRange};
use osdc::{ObjectOperation, PoolListError, ops::cmp_op};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

const FORMAT: &str = "rmds.format";
const IDENTITY: &str = "rmds.identity";
const AUTHORITY: &str = "rmds.authority";
const PHASE: &str = "rmds.phase";
// Bootstrap contains one empty root, not a production namespace checkpoint.
const MAX_PLAN: usize = 4 << 20;
const MAX_OBJECT: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapPlan {
    pub bootstrap_id: [u8; 16],
    pub identity: StoreIdentity,
    pub root_state: EngineCheckpoint,
    pub initial_range: InoRange,
    pub initial_checkpoint: CheckpointDescriptor,
    pub objects: Vec<BootstrapObject>,
    pub initial_owner: OwnerToken,
    pub initial_writer: WriterIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapObject {
    pub name: String,
    pub data: Bytes,
    pub xattrs: BTreeMap<String, Bytes>,
    pub omap: BTreeMap<Bytes, Bytes>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatPhase {
    Preparing,
    Ready,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormatInspection {
    pub revision: u64,
    pub writer: WriterIdentity,
    pub predecessor: Option<FencedWriter>,
    pub phase: FormatPhase,
    pub plan: BootstrapPlan,
}

impl BootstrapPlan {
    pub fn new(
        bootstrap_id: [u8; 16],
        identity: StoreIdentity,
        root_state: EngineCheckpoint,
        initial_range: InoRange,
        initial_owner: OwnerToken,
        initial_writer: WriterIdentity,
    ) -> Result<Self, StoreError> {
        let end = initial_range
            .start
            .checked_add(initial_range.len)
            .ok_or(StoreError::Capacity)?;
        if initial_range.len == 0
            || initial_range.start < 0x1000
            || end > (1 << 48)
            || initial_owner.rank != 0
            || initial_writer.rank_owner.rank != initial_owner.rank
            || initial_writer.rank_owner.incarnation != initial_owner.incarnation
            || initial_writer.rank_owner.gid != initial_owner.gid
            || initial_owner.shard_epoch == 0
            || initial_owner.incarnation == 0
            || initial_owner.gid == 0
            || root_state.inodes.len() != 1
            || root_state.inodes[0].core.ino != 1
            || !root_state.inodes[0].core.is_dir()
            || !root_state.inodes[0].entries.is_empty()
            || root_state.inodes[0].parent.is_some()
            || !root_state.inodes[0].remote_links.is_empty()
            || !root_state.sessions.is_empty()
            || !root_state.jobs.is_empty()
            || !root_state.cleanup_receipts.is_empty()
            || root_state.ino_alloc.free != [initial_range]
            || !root_state.ino_alloc.delegated.is_empty()
            || root_state.ino_alloc.floor != end
            || root_state
                .ino_alloc
                .free
                .iter()
                .chain(root_state.ino_alloc.delegated.iter().map(|(_, r)| r))
                .any(|r| {
                    r.start < initial_range.start
                        || r.start.checked_add(r.len).is_none_or(|n| n > end)
                })
        {
            return Err(StoreError::Corrupt);
        }
        let body = record::encode_checkpoint_bounded(&root_state, MAX_OBJECT - framing::OVERHEAD)?;
        let state = framing::encode(
            FrameKind::Checkpoint,
            &identity,
            initial_owner,
            root_state.covered_seq,
            [0; 16],
            &body,
            MAX_OBJECT,
        )?;
        let state_name = format!("rmds.checkpoint.0.{}.1.state", initial_owner.shard_epoch);
        let manifest_name = format!("rmds.checkpoint.0.{}.1.manifest", initial_owner.shard_epoch);
        let mut manifest = BytesMut::new();
        manifest.put_u64_le(1);
        put_blob(&mut manifest, state_name.as_bytes());
        manifest.put_u64_le(state.len() as u64);
        manifest.put_u32_le(framing::content_checksum(&state)?);
        let manifest = framing::encode(
            FrameKind::Manifest,
            &identity,
            initial_owner,
            root_state.covered_seq,
            [0; 16],
            &manifest,
            4096,
        )?;
        let initial_checkpoint = CheckpointDescriptor {
            generation: 1,
            covered_seq: root_state.covered_seq,
            manifest_name: manifest_name.clone(),
            manifest_checksum: framing::content_checksum(&manifest)?,
        };
        let authority = Authority {
            revision: 1,
            writer: initial_writer.clone(),
            predecessor: None,
        }
        .encode()?;
        let base = || {
            [(IDENTITY.to_owned(), identity.encode())]
                .into_iter()
                .collect::<BTreeMap<_, _>>()
        };
        let mut wal = base();
        wal.insert("rmds.owner".to_owned(), initial_owner.encode());
        wal.insert(AUTHORITY.to_owned(), authority.clone());
        wal.insert(
            "rmds.tail".to_owned(),
            Bytes::from(root_state.covered_seq.to_string()),
        );
        wal.insert(
            "rmds.trimmed".to_owned(),
            Bytes::from(root_state.covered_seq.to_string()),
        );
        wal.insert("rmds.checkpoint".to_owned(), initial_checkpoint.encode()?);
        let mut batch = BytesMut::new();
        if root_state.covered_seq != 0 {
            batch.extend_from_slice(&bootstrap_id);
            batch.put_u64_le(0);
            batch.put_u64_le(root_state.covered_seq);
            batch.put_u32_le(crc32c::crc32c(&body));
        }
        wal.insert("rmds.batch".to_owned(), batch.freeze());
        let mut sessions = base();
        sessions.insert(AUTHORITY.to_owned(), authority);
        sessions.insert("rmds.next_incarnation".to_owned(), Bytes::from_static(b"1"));
        let mut allocator = base();
        allocator.insert(
            "rmds.high_watermark".to_owned(),
            Bytes::from(end.to_string()),
        );
        let range = crate::allocator::encode_reservation(&identity, bootstrap_id, initial_range);
        let allocation_key = crate::allocator::reservation_key(bootstrap_id);
        let objects = vec![
            BootstrapObject {
                name: state_name,
                data: state,
                xattrs: BTreeMap::new(),
                omap: BTreeMap::new(),
            },
            BootstrapObject {
                name: manifest_name,
                data: manifest,
                xattrs: BTreeMap::new(),
                omap: BTreeMap::new(),
            },
            BootstrapObject {
                name: "rmds.inoalloc".to_owned(),
                data: Bytes::new(),
                xattrs: allocator,
                omap: [(allocation_key, range)].into_iter().collect(),
            },
            BootstrapObject {
                name: "rmds.wal.0.0".to_owned(),
                data: Bytes::new(),
                xattrs: wal,
                omap: BTreeMap::new(),
            },
            BootstrapObject {
                name: "rmds.sessions.0".to_owned(),
                data: Bytes::new(),
                xattrs: sessions,
                omap: BTreeMap::new(),
            },
            BootstrapObject {
                name: "rmds.shardmap".to_owned(),
                data: identity.encode(),
                xattrs: base(),
                omap: BTreeMap::new(),
            },
        ];
        Ok(Self {
            bootstrap_id,
            identity,
            root_state,
            initial_range,
            initial_checkpoint,
            objects,
            initial_owner,
            initial_writer,
        })
    }

    fn validate(&self) -> Result<(), StoreError> {
        let expected = Self::new(
            self.bootstrap_id,
            self.identity.clone(),
            self.root_state.clone(),
            self.initial_range,
            self.initial_owner,
            self.initial_writer.clone(),
        )?;
        if expected != *self {
            return Err(StoreError::Corrupt);
        }
        Ok(())
    }

    fn encode(&self) -> Result<Bytes, StoreError> {
        self.validate()?;
        let mut b = BytesMut::new();
        b.extend_from_slice(&self.bootstrap_id);
        b.put_u64_le(self.initial_range.start);
        b.put_u64_le(self.initial_range.len);
        crate::stable::put_writer(&mut b, &self.initial_writer)?;
        put_blob(&mut b, &record::encode_checkpoint(&self.root_state));
        put_blob(&mut b, &self.initial_checkpoint.encode()?);
        b.put_u32_le(self.objects.len() as u32);
        for object in &self.objects {
            let start = denc::encode_start(&mut b, 1, 1);
            put_blob(&mut b, object.name.as_bytes());
            put_blob(&mut b, &object.data);
            object.xattrs.encode(&mut b, 0);
            object.omap.encode(&mut b, 0);
            denc::encode_finish(&mut b, start);
        }
        framing::encode(
            FrameKind::Bootstrap,
            &self.identity,
            self.initial_owner,
            self.root_state.covered_seq,
            self.bootstrap_id,
            &b,
            MAX_PLAN,
        )
    }

    fn decode(bytes: &Bytes, identity: &StoreIdentity) -> Result<Self, StoreError> {
        // The framed owner/sequence are part of the protected plan. Read only the
        // fixed header to select expected sequence; decode verifies its checksum.
        if bytes.len() < framing::OVERHEAD {
            return Err(StoreError::Corrupt);
        }
        let mut seq = bytes.slice(80..88);
        let seq = seq.get_u64_le();
        let frame = framing::decode(bytes, FrameKind::Bootstrap, identity, seq, MAX_PLAN)?;
        let owner = frame.owner;
        let mut b = frame.body;
        if b.len() < 32 {
            return Err(StoreError::Corrupt);
        }
        let mut bootstrap_id = [0; 16];
        b.copy_to_slice(&mut bootstrap_id);
        if frame.transaction != bootstrap_id {
            return Err(StoreError::Corrupt);
        }
        let initial_range = InoRange {
            start: b.get_u64_le(),
            len: b.get_u64_le(),
        };
        let initial_writer = crate::stable::get_writer(&mut b)?;
        let root_state = record::decode_checkpoint(&blob(&mut b, MAX_OBJECT)?)
            .map_err(|_| StoreError::Corrupt)?;
        let initial_checkpoint =
            CheckpointDescriptor::decode(&blob(&mut b, 4096)?)?.ok_or(StoreError::Corrupt)?;
        if b.len() < 4 || b.get_u32_le() != 6 {
            return Err(StoreError::Corrupt);
        }
        let mut objects = Vec::new();
        for _ in 0..6 {
            let f = denc::decode_start(&mut b, 1, "BootstrapObject")
                .map_err(|_| StoreError::Corrupt)?;
            if f.v != 1 || f.compat != 1 {
                return Err(StoreError::UnsupportedVersion);
            }
            let mut f = f.body;
            let name =
                String::from_utf8(blob(&mut f, 256)?.to_vec()).map_err(|_| StoreError::Corrupt)?;
            let data = blob(&mut f, MAX_OBJECT)?;
            let attrs = decode_map(&mut f, 16, 4096)?;
            let mut xattrs = BTreeMap::new();
            for (k, v) in attrs {
                xattrs.insert(
                    String::from_utf8(k.to_vec()).map_err(|_| StoreError::Corrupt)?,
                    v,
                );
            }
            let omap = decode_map(&mut f, 16, MAX_OBJECT)?;
            if !f.is_empty() {
                return Err(StoreError::Corrupt);
            }
            objects.push(BootstrapObject {
                name,
                data,
                xattrs,
                omap,
            });
        }
        if !b.is_empty() || root_state.covered_seq != seq {
            return Err(StoreError::Corrupt);
        }
        let plan = Self {
            bootstrap_id,
            identity: identity.clone(),
            root_state,
            initial_range,
            initial_checkpoint,
            objects,
            initial_owner: owner,
            initial_writer,
        };
        plan.validate()?;
        Ok(plan)
    }
}

pub struct BootstrapStore<B> {
    backend: B,
    identity: StoreIdentity,
}

impl<B: Backend + PoolListing + Sync> BootstrapStore<B> {
    pub fn new(backend: B, identity: StoreIdentity) -> Self {
        Self { backend, identity }
    }

    pub async fn inspect_format(&self) -> Result<Option<FormatInspection>, StoreError> {
        let mut op = ObjectOperation::new();
        op.read(0, (MAX_PLAN + 1) as u64);
        let bytes = match checked(&self.backend, FORMAT, op).await {
            Ok(r) => r.out(0),
            Err(StoreError::MissingState) => return Ok(None),
            Err(e) => return Err(e),
        };
        if bytes.len() > MAX_PLAN {
            return Err(StoreError::Capacity);
        }
        let plan = BootstrapPlan::decode(&bytes, &self.identity)?;
        let mut op = ObjectOperation::new();
        op.getxattr(IDENTITY).getxattr(AUTHORITY).getxattr(PHASE);
        let r = checked(&self.backend, FORMAT, op).await?;
        if r.out(0) != self.identity.encode() {
            return Err(StoreError::WrongFormat);
        }
        let authority = Authority::decode(&r.out(1))?;
        let phase = match r.out(2).as_ref() {
            b"Preparing" => FormatPhase::Preparing,
            b"Ready" => FormatPhase::Ready,
            _ => return Err(StoreError::WrongFormat),
        };
        Ok(Some(FormatInspection {
            revision: authority.revision,
            writer: authority.writer,
            predecessor: authority.predecessor,
            phase,
            plan,
        }))
    }

    pub async fn begin_bootstrap(
        &self,
        plan: BootstrapPlan,
        writer: WriterIdentity,
    ) -> Result<FormatInspection, StoreError> {
        if plan.identity != self.identity || plan.initial_writer != writer {
            return Err(StoreError::WrongFormat);
        }
        let bytes = plan.encode()?;
        if let Some(existing) = self.inspect_format().await? {
            if existing.plan == plan
                && existing.writer == writer
                && existing.revision == 1
                && existing.predecessor.is_none()
                && existing.phase == FormatPhase::Preparing
            {
                return Ok(existing);
            }
            return Err(StoreError::WrongFormat);
        }
        self.scan(None).await?;
        let authority = Authority {
            revision: 1,
            writer,
            predecessor: None,
        };
        let mut op = ObjectOperation::new();
        op.create(true)
            .write_full(bytes)
            .setxattr(IDENTITY, self.identity.encode())
            .setxattr(AUTHORITY, authority.encode()?)
            .setxattr(PHASE, Bytes::from_static(b"Preparing"));
        match checked(&self.backend, FORMAT, op).await {
            Ok(_) | Err(StoreError::UnknownOutcome | StoreError::AlreadyExists) => {
                let current = self
                    .inspect_format()
                    .await?
                    .ok_or(StoreError::UnknownOutcome)?;
                if current.plan == plan && format_authority(&current) == authority {
                    Ok(current)
                } else {
                    Err(StoreError::WrongFormat)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub async fn acquire_preparing(
        &self,
        expected: &FormatInspection,
        writer: WriterIdentity,
        predecessor: FencedWriter,
    ) -> Result<FormatInspection, StoreError> {
        if expected.phase != FormatPhase::Preparing
            || predecessor.writer != expected.writer
            || predecessor.verified_osd_epoch == 0
            || writer.rank_owner.rank != expected.writer.rank_owner.rank
            || writer.rank_owner.incarnation <= expected.writer.rank_owner.incarnation
            || writer.rank_owner.gid == expected.writer.rank_owner.gid
        {
            return Err(StoreError::Fenced);
        }
        let mut next = expected.clone();
        next.writer = writer;
        next.predecessor = Some(predecessor);
        next.revision = next.revision.checked_add(1).ok_or(StoreError::Capacity)?;
        let actual = self
            .inspect_format()
            .await?
            .ok_or(StoreError::MissingState)?;
        if actual == next {
            return Ok(actual);
        }
        if actual != *expected {
            return Err(StoreError::Fenced);
        }
        let mut op = self.guard(expected)?;
        op.setxattr(AUTHORITY, format_authority(&next).encode()?);
        match checked(&self.backend, FORMAT, op).await {
            Ok(_) | Err(StoreError::UnknownOutcome | StoreError::GuardRejected) => {
                let actual = self
                    .inspect_format()
                    .await?
                    .ok_or(StoreError::MissingState)?;
                if actual == next {
                    Ok(actual)
                } else if actual == *expected {
                    Err(StoreError::UnknownOutcome)
                } else {
                    Err(StoreError::Fenced)
                }
            }
            Err(e) => Err(e),
        }
    }

    fn guard(&self, expected: &FormatInspection) -> Result<ObjectOperation, StoreError> {
        let mut op = ObjectOperation::new();
        op.cmpxattr_str(IDENTITY, cmp_op::EQ, &self.identity.encode());
        op.cmpxattr_str(AUTHORITY, cmp_op::EQ, &format_authority(expected).encode()?);
        op.cmpxattr_str(
            PHASE,
            cmp_op::EQ,
            match expected.phase {
                FormatPhase::Preparing => b"Preparing",
                FormatPhase::Ready => b"Ready",
            },
        );
        Ok(op)
    }

    /// Verify/create the persisted plan. Existing shard/session objects must
    /// already have this preparer's independent authorities. A different writer
    /// returns Fenced; the daemon must inspect/fence/acquire that object explicitly.
    pub async fn resume_bootstrap(
        &self,
        format: &FormatInspection,
        owner: OwnerToken,
    ) -> Result<OpenedShard, StoreError> {
        if format.phase != FormatPhase::Preparing
            || owner.rank != format.writer.rank_owner.rank
            || owner.incarnation != format.writer.rank_owner.incarnation
            || owner.gid != format.writer.rank_owner.gid
        {
            return Err(StoreError::Fenced);
        }
        let actual = self
            .inspect_format()
            .await?
            .ok_or(StoreError::MissingState)?;
        if actual != *format {
            return Err(StoreError::Fenced);
        }
        self.scan(Some(&format.plan)).await?;
        for object in &format.plan.objects {
            let mut op = ObjectOperation::new();
            op.create(true).write_full(object.data.clone());
            for (name, value) in &object.xattrs {
                op.setxattr(name, value.clone());
            }
            if !object.omap.is_empty() {
                op.omap_set_vals(&object.omap);
            }
            match checked(&self.backend, &object.name, op).await {
                Ok(_) | Err(StoreError::AlreadyExists | StoreError::UnknownOutcome) => {
                    self.verify_object(object, true).await?
                }
                Err(e) => return Err(e),
            }
        }
        self.verify_guards(format, owner).await
    }

    async fn verify_guards(
        &self,
        format: &FormatInspection,
        owner: OwnerToken,
    ) -> Result<OpenedShard, StoreError> {
        // Borrow the backend. This adapter does not create another native client.
        let store = StableStore::new(
            Borrowed(&self.backend),
            self.identity.clone(),
            Limits::default(),
        )?;
        let opened = store.inspect_frozen().await?;
        if opened.owner != owner {
            return Err(StoreError::Fenced);
        }
        let sessions = crate::sessions::SessionRepository::new(
            Borrowed(&self.backend),
            self.identity.clone(),
            owner.rank,
            1,
        )?;
        let sessions = sessions.inspect_sessions().await?;
        if sessions.writer != format.writer {
            return Err(StoreError::Fenced);
        }
        if sessions.next_incarnation != 1
            || opened.tail != format.plan.root_state.covered_seq
            || opened.checkpoint != Some(format.plan.initial_checkpoint.clone())
        {
            return Err(StoreError::Corrupt);
        }
        Ok(OpenedShard::from_frozen(opened))
    }

    pub async fn publish_ready(
        &self,
        expected: &FormatInspection,
    ) -> Result<FormatInspection, StoreError> {
        let actual = self
            .inspect_format()
            .await?
            .ok_or(StoreError::MissingState)?;
        let mut desired = expected.clone();
        desired.phase = FormatPhase::Ready;
        desired.revision = desired
            .revision
            .checked_add(1)
            .ok_or(StoreError::Capacity)?;
        if actual == desired {
            return Ok(actual);
        }
        if actual != *expected || expected.phase != FormatPhase::Preparing {
            return Err(StoreError::Fenced);
        }
        let wal = expected
            .plan
            .objects
            .iter()
            .find(|o| o.name == "rmds.wal.0.0")
            .ok_or(StoreError::Corrupt)?;
        let mut op = ObjectOperation::new();
        op.getxattr("rmds.owner");
        let reply = checked(&self.backend, &wal.name, op).await?;
        let owner = OwnerToken::decode(&reply.out(0))?;
        for object in &expected.plan.objects {
            self.verify_object(object, true).await?;
        }
        self.verify_guards(expected, owner).await?;
        if owner.rank != expected.writer.rank_owner.rank
            || owner.incarnation != expected.writer.rank_owner.incarnation
            || owner.gid != expected.writer.rank_owner.gid
        {
            return Err(StoreError::Fenced);
        }
        let mut op = self.guard(expected)?;
        op.setxattr(AUTHORITY, format_authority(&desired).encode()?)
            .setxattr(PHASE, Bytes::from_static(b"Ready"));
        match checked(&self.backend, FORMAT, op).await {
            Ok(_) | Err(StoreError::UnknownOutcome | StoreError::GuardRejected) => {
                let current = self
                    .inspect_format()
                    .await?
                    .ok_or(StoreError::MissingState)?;
                if current == desired {
                    Ok(current)
                } else if current == *expected {
                    Err(StoreError::UnknownOutcome)
                } else {
                    Err(StoreError::Fenced)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Call after fencing/acquisition and before namespace service. Ready with a
    /// missing dependency is damage. Mutable repositories retain their current
    /// positions; this check never compares them to their bootstrap counters.
    pub async fn validate_ready(&self, expected: &FormatInspection) -> Result<(), StoreError> {
        if expected.phase != FormatPhase::Ready {
            return Err(StoreError::MissingState);
        }
        if self.inspect_format().await?.as_ref() != Some(expected) {
            return Err(StoreError::Fenced);
        }
        for object in &expected.plan.objects {
            if object.name.starts_with("rmds.checkpoint.") {
                self.verify_object(object, false).await?;
            } else {
                let mut op = ObjectOperation::new();
                op.getxattr(IDENTITY);
                let reply = checked(&self.backend, &object.name, op).await?;
                if reply.out(0) != self.identity.encode() {
                    return Err(StoreError::WrongFormat);
                }
            }
        }
        // Validate the initial allocator receipt, even after later reservations.
        let allocator = expected
            .plan
            .objects
            .iter()
            .find(|o| o.name == "rmds.inoalloc")
            .ok_or(StoreError::Corrupt)?;
        let mut op = ObjectOperation::new();
        op.getxattr("rmds.high_watermark")
            .omap_get_vals_by_keys(&allocator.omap.keys().cloned().collect());
        let reply = checked(&self.backend, &allocator.name, op).await?;
        let high = reply.out(0);
        let high = std::str::from_utf8(&high)
            .map_err(|_| StoreError::Corrupt)?
            .parse::<u64>()
            .map_err(|_| StoreError::Corrupt)?;
        if high < expected.plan.initial_range.start + expected.plan.initial_range.len
            || high > (1 << 48)
        {
            return Err(StoreError::Corrupt);
        }
        let mut output = reply.out(1);
        if decode_map(&mut output, 1, 128)? != allocator.omap || !output.is_empty() {
            return Err(StoreError::Corrupt);
        }
        Ok(())
    }

    async fn scan(&self, plan: Option<&BootstrapPlan>) -> Result<(), StoreError> {
        let mut cursor = None;
        let mut restarts = 0;
        let mut pages = 0;
        let start = Instant::now();
        let allowed: BTreeSet<_> = plan
            .map(|p| {
                p.objects
                    .iter()
                    .map(|o| o.name.as_bytes())
                    .chain([FORMAT.as_bytes()])
                    .collect()
            })
            .unwrap_or_default();
        loop {
            if pages >= 4096 || start.elapsed() > Duration::from_secs(60) {
                return Err(StoreError::Capacity);
            }
            pages += 1;
            let page = match self.backend.list_pool_page(cursor.clone(), 64, 65536).await {
                Ok(p) => p,
                Err(PoolListError::MapChanged) if restarts < 4 => {
                    restarts += 1;
                    cursor = None;
                    continue;
                }
                Err(PoolListError::Unsupported) => return Err(StoreError::UnsupportedVersion),
                Err(_) => return Err(StoreError::Io),
            };
            if page.objects.len() > 64
                || page
                    .objects
                    .iter()
                    .map(|o| o.name.len() + o.namespace.len() + o.locator.len())
                    .sum::<usize>()
                    > 65536
            {
                return Err(StoreError::Capacity);
            }
            for object in page.objects {
                if !object.namespace.is_empty()
                    || !object.locator.is_empty()
                    || !allowed.contains(object.name.as_slice())
                {
                    return Err(StoreError::WrongFormat);
                }
                if let Some(planned) =
                    plan.and_then(|p| p.objects.iter().find(|o| o.name.as_bytes() == object.name))
                {
                    self.verify_object(planned, true).await?;
                }
            }
            if page.next.is_some() && page.next == cursor {
                return Err(StoreError::Corrupt);
            }
            cursor = page.next;
            if cursor.is_none() {
                return Ok(());
            }
        }
    }

    async fn verify_object(
        &self,
        object: &BootstrapObject,
        acquired: bool,
    ) -> Result<(), StoreError> {
        let authority_object = object.name == "rmds.wal.0.0" || object.name == "rmds.sessions.0";
        let attrs: Vec<_> = object
            .xattrs
            .iter()
            .filter(|(name, _)| {
                !(acquired
                    && authority_object
                    && (name.as_str() == AUTHORITY || name.as_str() == "rmds.owner"))
            })
            .collect();
        let mut op = ObjectOperation::new();
        op.read(0, object.data.len() as u64 + 1);
        for (name, _) in &attrs {
            op.getxattr(name);
        }
        op.omap_get_vals(b"", b"", object.omap.len() as u64 + 1);
        let r = checked(&self.backend, &object.name, op).await?;
        if r.out(0) != object.data {
            return Err(StoreError::Corrupt);
        }
        for (i, (_, value)) in attrs.iter().enumerate() {
            if r.out(i + 1) != **value {
                return Err(StoreError::Corrupt);
            }
        }
        let mut output = r.out(attrs.len() + 1);
        let values = decode_map(&mut output, object.omap.len() + 1, MAX_OBJECT)?;
        if output.as_ref() != [0] || values != object.omap {
            return Err(StoreError::Corrupt);
        }
        Ok(())
    }
}

struct Borrowed<'a, B>(&'a B);
impl<B: Backend + Sync> Backend for Borrowed<'_, B> {
    async fn operate(&self, name: &str, op: ObjectOperation) -> Result<osdc::OpReply, StoreError> {
        self.0.operate(name, op).await
    }
}

fn format_authority(format: &FormatInspection) -> Authority {
    Authority {
        revision: format.revision,
        writer: format.writer.clone(),
        predecessor: format.predecessor.clone(),
    }
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
fn decode_map(
    b: &mut Bytes,
    max: usize,
    max_value: usize,
) -> Result<BTreeMap<Bytes, Bytes>, StoreError> {
    if b.len() < 4 {
        return Err(StoreError::Corrupt);
    }
    let n = b.get_u32_le() as usize;
    if n > max {
        return Err(StoreError::Capacity);
    }
    let mut values = BTreeMap::new();
    for _ in 0..n {
        let key = blob(b, 256)?;
        let value = blob(b, max_value)?;
        if values.insert(key, value).is_some() {
            return Err(StoreError::Corrupt);
        }
    }
    Ok(values)
}
