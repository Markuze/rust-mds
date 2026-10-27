//! Materialized checkpoints, checked ordered replay, and durable data work.

use super::*;
use crate::ino::InoAllocState;

impl<J: Journal> Engine<J> {
    pub fn install_session_key(&mut self, key: SessionKey) -> Result<(), RestoreError> {
        if self.is_failed() {
            return Err(RestoreError::Failed);
        }
        if key.incarnation == 0 || self.cleanup_receipts.contains_key(&key) {
            return Err(RestoreError::Invalid("retired or zero session key"));
        }
        if let Some(s) = self.sessions.get(&key.client) {
            if s.epoch != key.incarnation {
                return Err(RestoreError::Invalid("live incarnation mismatch"));
            }
        } else {
            self.sessions
                .insert(key.client, Session::with_epoch(key.client, key.incarnation));
        }
        self.next_session_epoch = self.next_session_epoch.max(key.incarnation);
        Ok(())
    }
    pub fn session_cleanup_seq(&self, key: SessionKey) -> Option<JournalSeq> {
        self.cleanup_receipts.get(&key).copied()
    }
    pub fn submitted_seq(&self) -> JournalSeq {
        self.submitted_seq
    }
    pub fn durable_seq(&self) -> JournalSeq {
        self.durable_seq
    }
    fn recovery_covers_range(core: &InodeCore, bound: u64, job: &LifecycleIntent) -> bool {
        job.ino == core.ino
            && matches!(&job.kind,
            LifecycleKind::RecoverSize { max_size, inode_version, truncate_seq, .. }
                if *max_size >= bound && *inode_version == core.version
                    && *truncate_seq == core.truncate_seq)
    }
    pub(super) fn writer_range_is_covered(&self, ino: Ino, client: ClientId) -> bool {
        let Some(i) = self.inodes.get(&ino) else {
            return true;
        };
        let Some(bound) = i.core.client_ranges.get(&client) else {
            return true;
        };
        self.jobs
            .values()
            .any(|job| Self::recovery_covers_range(&i.core, *bound, job))
    }
    fn state_image(&self) -> EngineCheckpoint {
        EngineCheckpoint {
            recovery_semantics: self.recovery_semantics,
            covered_seq: self.durable_seq,
            inodes: self
                .inodes
                .values()
                .map(|i| CheckpointInode {
                    core: i.core.clone(),
                    entries: i.entries.iter().map(|(k, v)| (k.clone(), *v)).collect(),
                    parent: i.parent.clone(),
                    remote_links: i.remote_links.iter().cloned().collect(),
                })
                .collect(),
            ino_alloc: self.ino_alloc.state(),
            sessions: self
                .sessions
                .values()
                .map(|s| CheckpointSession {
                    key: SessionKey {
                        client: s.client,
                        incarnation: s.epoch,
                    },
                    completed_requests: s.completed_requests.values().cloned().collect(),
                    completed_flushes: s.completed_flushes.iter().copied().collect(),
                    oldest_client_tid: s.durable_oldest_client_tid,
                    oldest_flush_tid: s.oldest_flush_tid,
                })
                .collect(),
            jobs: self.jobs.values().cloned().collect(),
            next_job_id: self.next_job_id,
            last_cap_id: self.last_cap_id,
            cleanup_receipts: self
                .cleanup_receipts
                .iter()
                .map(|(k, s)| (*k, *s))
                .collect(),
        }
    }
    pub fn checkpoint(&self) -> Result<EngineCheckpoint, CheckpointError> {
        if self.is_failed() {
            return Err(CheckpointError::Failed);
        }
        if !self.is_quiescent()
            || self.submitted_seq != self.durable_seq
            || self.journal.committed_seq() != self.durable_seq
            || self
                .inodes
                .values()
                .any(|i| i.pending != 0 || !i.holds.is_empty())
        {
            return Err(CheckpointError::Busy);
        }
        Ok(self.state_image())
    }
    pub fn restore(
        config: EngineConfig,
        journal: J,
        state: EngineCheckpoint,
    ) -> Result<Self, RestoreError> {
        Self::validate_state(&state)?;
        InoAllocator::new()
            .add_lease(config.ino_lease)
            .map_err(|_| RestoreError::Invalid("configuration inode lease"))?;
        let mut e = Self::empty(config, journal);
        if !e.valid_retry_limits() {
            return Err(RestoreError::Capacity);
        }
        e.recovery_semantics = state.recovery_semantics;
        e.submitted_seq = state.covered_seq;
        e.durable_seq = state.covered_seq;
        e.last_cap_id = state.last_cap_id;
        e.next_job_id = state.next_job_id;
        e.recovery_phase = RecoveryPhase::Reconstruction;
        for i in state.inodes {
            let mut n = Inode::new(i.core.clone());
            n.entries = i.entries.into_iter().collect();
            n.parent = i.parent;
            n.remote_links = i.remote_links.into_iter().collect();
            e.committed_inodes.insert(i.core.ino, i.core);
            e.inodes.insert(n.ino(), n);
        }
        e.ino_alloc = InoAllocator::restore(&state.ino_alloc);
        for s in state.sessions {
            e.install_session_key(s.key)?;
            let session = e
                .sessions
                .get_mut(&s.key.client)
                .ok_or(RestoreError::Invalid("session"))?;
            session.completed_requests = s
                .completed_requests
                .into_iter()
                .map(|r| (r.tid, r))
                .collect();
            session.completed_flushes = s.completed_flushes.into_iter().collect();
            session.oldest_client_tid = s.oldest_client_tid;
            session.durable_oldest_client_tid = s.oldest_client_tid;
            session.oldest_flush_tid = s.oldest_flush_tid;
        }
        e.jobs = state.jobs.into_iter().map(|j| (j.id, j)).collect();
        e.cleanup_receipts = state.cleanup_receipts.into_iter().collect();
        e.next_session_epoch = e.next_session_epoch.max(
            e.cleanup_receipts
                .keys()
                .map(|k| k.incarnation)
                .max()
                .unwrap_or(0),
        );
        e.rebuild_retry_refs();
        if e.recovery_semantics == RecoverySemantics::RetainedTargets3
            && !e.retry_charge().fits(e.config.retry_limits)
        {
            return Err(RestoreError::Capacity);
        }
        Ok(e)
    }
    /// Export a checked clean Legacy2 load for store-controlled adoption.
    /// This does not publish state or release either recovery gate.
    pub fn checkpoint_for_retry_upgrade(&self) -> Result<EngineCheckpoint, RestoreError> {
        if self.is_failed() {
            return Err(RestoreError::Failed);
        }
        if self.recovery_semantics != RecoverySemantics::Legacy2
            || !self.is_quiescent()
            || !self.sessions.is_empty()
            || self.submitted_seq != self.durable_seq
            || self.journal.committed_seq() != self.durable_seq
            || self
                .inodes
                .values()
                .any(|i| i.pending != 0 || !i.holds.is_empty())
        {
            return Err(RestoreError::UpgradeRequired(
                "legacy source is not a clean load",
            ));
        }
        let mut state = self.state_image();
        Self::validate_state(&state)?;
        state.recovery_semantics = RecoverySemantics::RetainedTargets3;
        if !self.retry_charge().fits(self.config.retry_limits) {
            return Err(RestoreError::Capacity);
        }
        Self::validate_state(&state)?;
        Ok(state)
    }
    fn validate_allocator(a: &InoAllocState, inodes: &BTreeSet<Ino>) -> Result<(), RestoreError> {
        if a.free.windows(2).any(|w| w[0].start >= w[1].start) {
            return Err(RestoreError::Invalid("unsorted free ranges"));
        }
        if a.floor > INO_LIMIT {
            return Err(RestoreError::Invalid("allocator floor"));
        }
        if inodes
            .iter()
            .any(|ino| *ino != CEPH_INO_ROOT && *ino >= a.floor)
        {
            return Err(RestoreError::Invalid(
                "allocated inode beyond allocator floor",
            ));
        }
        let mut ranges: Vec<_> = a
            .free
            .iter()
            .chain(a.delegated.iter().map(|(_, r)| r))
            .copied()
            .collect();
        ranges.sort_by_key(|r| r.start);
        let mut end = 0;
        for r in ranges {
            let Some(next) = r.start.checked_add(r.len) else {
                return Err(RestoreError::Invalid("allocator overflow"));
            };
            if r.len == 0
                || r.start < MDS_INO_SYSTEM_BASE
                || next > a.floor
                || r.start < end
                || inodes.range(r.start..next).next().is_some()
            {
                return Err(RestoreError::Invalid("allocator overlap/range"));
            }
            end = next;
        }
        Ok(())
    }
    fn validate_state(s: &EngineCheckpoint) -> Result<(), RestoreError> {
        let bad = RestoreError::Invalid;
        let mut nodes = BTreeMap::new();
        for i in &s.inodes {
            if nodes.insert(i.core.ino, i).is_some() {
                return Err(bad("duplicate inode"));
            }
            if i.core.ino >= INO_LIMIT
                || (i.core.ino != CEPH_INO_ROOT && i.core.ino < MDS_INO_SYSTEM_BASE)
                || i.core.version == 0
                || i.core.version >= u64::MAX / 2
                || i.core.nlink < 0
                || !i.core.export_pin_random.is_finite()
                || i.core.stable_subvolume_id >= INO_LIMIT
            {
                return Err(bad("inode fields"));
            }
            if i.core.nlink == 0
                && (i.parent.is_some()
                    || i.core
                        .stray_auth_path
                        .as_ref()
                        .is_none_or(|p| !p.starts_with(b"/")))
            {
                return Err(bad("stray authorization path"));
            }
            if i.core.is_dir() && (i.core.nlink > 1 || i.core.size != 0) {
                return Err(bad("directory fields"));
            }
            if i.core.is_file()
                && (!i.core.layout.is_valid_geometry()
                    || i.core.client_ranges.values().any(|r| *r == 0))
            {
                return Err(bad("layout or writable range"));
            }
            if i.core.old_layouts.iter().any(|l| !l.is_valid_geometry()) {
                return Err(bad("old layout"));
            }
        }
        let root = nodes.get(&CEPH_INO_ROOT).ok_or(bad("missing root"))?;
        if !root.core.is_dir() || root.parent.is_some() || root.core.nlink != 1 {
            return Err(bad("root"));
        }
        let mut primary: BTreeMap<Ino, (Ino, Vec<u8>)> = BTreeMap::new();
        let mut remotes: BTreeMap<Ino, BTreeSet<(Ino, Vec<u8>)>> = BTreeMap::new();
        for i in &s.inodes {
            if !i.core.is_dir() && !i.entries.is_empty() {
                return Err(bad("file dentries"));
            }
            let mut seen = BTreeSet::new();
            for (key, d) in &i.entries {
                if key.name.is_empty()
                    || key.name.contains(&b'/')
                    || key.name == b"."
                    || key.name == b".."
                    || key != &crate::inode::DentryKey::new(i.core.dir_hash, &key.name)
                    || !seen.insert(key)
                    || !nodes.contains_key(&d.ino)
                    || d.ino == CEPH_INO_ROOT
                {
                    return Err(bad("dentry"));
                }
                let link = (i.core.ino, key.name.clone());
                if d.primary {
                    if primary.insert(d.ino, link).is_some() {
                        return Err(bad("duplicate primary"));
                    }
                } else {
                    remotes.entry(d.ino).or_default().insert(link);
                }
            }
        }
        for i in &s.inodes {
            let ino = i.core.ino;
            if i.parent.as_ref() != primary.get(&ino) {
                return Err(bad("primary linkage"));
            }
            let remote: BTreeSet<_> = i.remote_links.iter().cloned().collect();
            if remote.len() != i.remote_links.len()
                || remote != remotes.get(&ino).cloned().unwrap_or_default()
            {
                return Err(bad("remote linkage"));
            }
            let links = usize::from(i.parent.is_some()) + remote.len();
            if ino != CEPH_INO_ROOT && links != i.core.nlink as usize {
                return Err(bad("link count"));
            }
            if i.core.is_dir() && !remote.is_empty() {
                return Err(bad("directory remote linkage"));
            }
            if ino != CEPH_INO_ROOT && i.core.nlink > 0 && i.parent.is_none() {
                return Err(bad("linked inode without primary"));
            }
            let mut cur = Some(ino);
            let mut ancestors = BTreeSet::new();
            let mut rooted = false;
            while let Some(n) = cur {
                if !ancestors.insert(n) {
                    return Err(bad("parent cycle"));
                }
                if n == CEPH_INO_ROOT {
                    rooted = true;
                    break;
                }
                cur = nodes
                    .get(&n)
                    .and_then(|i| i.parent.as_ref().map(|(p, _)| *p));
            }
            if i.core.nlink > 0 && !rooted {
                return Err(bad("unrooted primary chain"));
            }
            if i.core.is_dir() {
                let files = i
                    .entries
                    .iter()
                    .filter(|(_, d)| !nodes[&d.ino].core.is_dir())
                    .count() as i64;
                let dirs = i.entries.len() as i64 - files;
                if i.core.dirstat.nfiles != files || i.core.dirstat.nsubdirs != dirs {
                    return Err(bad("directory statistics"));
                }
            }
        }
        let reserved: BTreeSet<_> = nodes
            .keys()
            .copied()
            .chain(s.jobs.iter().map(|j| j.ino))
            .collect();
        Self::validate_allocator(&s.ino_alloc, &reserved)?;
        let mut keys = BTreeSet::new();
        let mut clients = BTreeSet::new();
        for session in &s.sessions {
            if session.key.incarnation == 0
                || !keys.insert(session.key)
                || !clients.insert(session.key.client)
            {
                return Err(bad("session key"));
            }
            let mut tids = BTreeSet::new();
            for r in &session.completed_requests {
                if r.client != session.key.client
                    || r.session_incarnation != session.key.incarnation
                    || r.tid < session.oldest_client_tid
                    || !tids.insert(r.tid)
                {
                    return Err(bad("request history"));
                }
                if s.recovery_semantics == RecoverySemantics::RetainedTargets3 {
                    if r.result < 0
                        || !matches!(
                            r.op,
                            CEPH_MDS_OP_CREATE
                                | CEPH_MDS_OP_OPEN
                                | CEPH_MDS_OP_MKNOD
                                | CEPH_MDS_OP_MKDIR
                                | CEPH_MDS_OP_SYMLINK
                                | CEPH_MDS_OP_LINK
                                | CEPH_MDS_OP_UNLINK
                                | CEPH_MDS_OP_RMDIR
                                | CEPH_MDS_OP_RENAME
                                | CEPH_MDS_OP_SETATTR
                                | CEPH_MDS_OP_SETXATTR
                                | CEPH_MDS_OP_RMXATTR
                        )
                    {
                        return Err(bad("unsupported completion outcome or operation"));
                    }
                    let creator = matches!(
                        r.op,
                        CEPH_MDS_OP_CREATE
                            | CEPH_MDS_OP_MKNOD
                            | CEPH_MDS_OP_MKDIR
                            | CEPH_MDS_OP_SYMLINK
                    );
                    if r.created_ino
                        .is_some_and(|ino| !creator || ino < MDS_INO_SYSTEM_BASE)
                    {
                        return Err(bad("unexpected completion creation"));
                    }
                    if r.oldest_client_tid > r.tid {
                        return Err(bad("completion watermark"));
                    }
                    if super::retry::target_operation(r.op) {
                        let target = r
                            .reply_target_ino
                            .ok_or(bad("missing retained reply target"))?;
                        let core = &nodes
                            .get(&target)
                            .ok_or(bad("missing retained reply target"))?
                            .core;
                        let valid_kind = match r.op {
                            CEPH_MDS_OP_CREATE | CEPH_MDS_OP_OPEN => core.is_file(),
                            CEPH_MDS_OP_MKDIR => core.is_dir(),
                            CEPH_MDS_OP_SYMLINK => core.is_symlink(),
                            // MKNOD also delegates directory creation and can
                            // create other supported node kinds.
                            CEPH_MDS_OP_MKNOD => true,
                            _ => false,
                        };
                        if !valid_kind
                            || r.created_ino.is_some_and(|created| created != target)
                            || (matches!(
                                r.op,
                                CEPH_MDS_OP_MKNOD | CEPH_MDS_OP_MKDIR | CEPH_MDS_OP_SYMLINK
                            ) && r.created_ino != Some(target))
                            || s.jobs.iter().any(|j| {
                                j.ino == target && matches!(j.kind, LifecycleKind::Purge { .. })
                            })
                        {
                            return Err(bad("invalid retained reply target"));
                        }
                    } else if r.reply_target_ino.is_some() {
                        return Err(bad("unexpected retained reply target"));
                    }
                }
            }
            let flushes: BTreeSet<_> = session.completed_flushes.iter().copied().collect();
            if flushes.len() != session.completed_flushes.len()
                || flushes
                    .first()
                    .is_some_and(|t| *t < session.oldest_flush_tid)
            {
                return Err(bad("flush history"));
            }
        }
        let mut receipts = BTreeSet::new();
        for (k, seq) in &s.cleanup_receipts {
            if k.incarnation == 0
                || *seq == 0
                || *seq > s.covered_seq
                || keys.contains(k)
                || !receipts.insert(*k)
            {
                return Err(bad("cleanup receipt"));
            }
        }
        let mut jobs = BTreeSet::new();
        for j in &s.jobs {
            if j.id == 0 || j.id >= s.next_job_id || !jobs.insert(j.id) {
                return Err(bad("job id"));
            }
            match &j.kind {
                LifecycleKind::Purge { layouts, .. } => {
                    if nodes.contains_key(&j.ino)
                        || layouts.is_empty()
                        || layouts.iter().any(|l| l.is_set() && !l.is_valid_geometry())
                    {
                        return Err(bad("purge job"));
                    }
                }
                LifecycleKind::Truncate {
                    layout,
                    size,
                    from,
                    truncate_seq,
                } => {
                    if !nodes.contains_key(&j.ino) || !layout.is_valid_geometry() || size > from {
                        return Err(bad("truncate job"));
                    }
                    if *truncate_seq == 0 || *truncate_seq > nodes[&j.ino].core.truncate_seq {
                        return Err(bad("truncate sequence"));
                    }
                }
                LifecycleKind::RecoverSize {
                    layouts,
                    max_size,
                    inode_version,
                    truncate_seq,
                } => {
                    if !nodes.contains_key(&j.ino)
                        || layouts.is_empty()
                        || layouts.iter().any(|l| !l.is_valid_geometry())
                    {
                        return Err(bad("recovery job"));
                    }
                    let core = &nodes[&j.ino].core;
                    if core.version != *inode_version
                        || core.truncate_seq != *truncate_seq
                        || core.client_ranges.is_empty()
                        || core.max_client_range() > *max_size
                    {
                        return Err(bad("recovery guard/range"));
                    }
                }
            }
        }
        // A retained bound must have a live owner or a durable recovery path.
        // An old receipt does not retire a newer live key of the same client.
        // Apply this after job validation, both to checkpoints and to every
        // replay candidate, including records that only retire lifecycle jobs.
        for inode in &s.inodes {
            for (client, bound) in &inode.core.client_ranges {
                if !clients.contains(client)
                    && !s
                        .jobs
                        .iter()
                        .any(|job| Self::recovery_covers_range(&inode.core, *bound, job))
                {
                    return Err(bad("writer range without live owner or recovery intent"));
                }
            }
        }
        if s.next_job_id == 0
            || s.next_job_id == u64::MAX
            || s.last_cap_id == u64::MAX
            || s.covered_seq == u64::MAX
        {
            return Err(bad("exhausted counter"));
        }
        Ok(())
    }
    /// Below the previous high watermark, a range can use only still-available
    /// source ranges. A higher floor authorizes newly reserved space, not reuse.
    fn validate_range_provenance(
        range: InoRange,
        floor: Ino,
        sources: impl Iterator<Item = InoRange>,
    ) -> Result<(), RestoreError> {
        let end = range
            .start
            .checked_add(range.len)
            .ok_or(RestoreError::Invalid("allocator overflow"))?
            .min(floor);
        let mut cursor = range.start;
        let mut sources: Vec<_> = sources.collect();
        sources.sort_by_key(|r| r.start);
        for source in sources {
            if cursor >= end {
                break;
            }
            let source_end = source
                .start
                .checked_add(source.len)
                .ok_or(RestoreError::Invalid("allocator source overflow"))?;
            if source.start <= cursor && source_end > cursor {
                cursor = source_end;
            }
        }
        if cursor < end {
            return Err(RestoreError::Invalid("allocator range provenance"));
        }
        Ok(())
    }
    pub fn replay_record(
        &mut self,
        seq: JournalSeq,
        record: &JournalRecord,
    ) -> Result<(), RestoreError> {
        if self.is_failed() {
            return Err(RestoreError::Failed);
        }
        if record.recovery_semantics != self.recovery_semantics {
            return Err(RestoreError::UpgradeRequired("mixed recovery semantics"));
        }
        if self.recovery_phase != RecoveryPhase::Reconstruction
            || !self.is_quiescent()
            || self
                .inodes
                .values()
                .any(|i| !i.caps.is_empty() || i.has_file_locks())
        {
            return Err(RestoreError::Invalid("replay while active"));
        }
        let expected = self
            .durable_seq
            .checked_add(1)
            .ok_or(RestoreError::Invalid("sequence overflow"))?;
        if seq != expected {
            return Err(RestoreError::Sequence {
                expected,
                actual: seq,
            });
        }
        if let Some(a) = &record.ino_alloc {
            let old = self.ino_alloc.state();
            if a.floor < old.floor {
                return Err(RestoreError::Invalid("allocator floor regression"));
            }
            for r in &a.free {
                Self::validate_range_provenance(*r, old.floor, old.free.iter().copied())?;
            }
            for (owner, range) in &a.delegated {
                Self::validate_range_provenance(
                    *range,
                    old.floor,
                    old.free.iter().copied().chain(
                        old.delegated
                            .iter()
                            .filter(|(client, _)| client == owner)
                            .map(|(_, r)| *r),
                    ),
                )?;
            }
        }
        let mut allocated = BTreeSet::new();
        for ino in &record.allocated {
            if !allocated.insert(*ino)
                || record.ino_alloc.is_none()
                || self.inodes.contains_key(ino)
                || (!self.ino_alloc.is_free(*ino) && *ino < self.ino_alloc.state().floor)
            {
                return Err(RestoreError::Invalid("allocation reuse/reference"));
            }
        }
        if record.closed_sessions.iter().collect::<BTreeSet<_>>().len()
            != record.closed_sessions.len()
            || record.completed_jobs.iter().collect::<BTreeSet<_>>().len()
                != record.completed_jobs.len()
        {
            return Err(RestoreError::Invalid("duplicate retirement"));
        }
        if self.recovery_semantics == RecoverySemantics::RetainedTargets3
            && let Some(done) = &record.completed_request
            && (done.oldest_client_tid > done.tid
                || self.sessions.get(&done.client).is_some_and(|s| {
                    s.completed_requests.contains_key(&done.tid)
                        || done.tid < s.durable_oldest_client_tid
                }))
        {
            return Err(RestoreError::Invalid(
                "repeated or retired completion identity",
            ));
        }
        let mut candidate = Engine::<crate::MemJournal>::restore(
            self.config.clone(),
            crate::MemJournal::at_seq(self.durable_seq, false),
            self.state_image(),
        )?;
        let mut imaged = BTreeSet::new();
        for core in &record.inodes {
            if !imaged.insert(core.ino) {
                return Err(RestoreError::Invalid("duplicate image"));
            }
            if let Some(old) = self.inodes.get(&core.ino) {
                if core.version <= old.core.version
                    || core.mode & S_IFMT != old.core.mode & S_IFMT
                    || (old.core.stable_subvolume_id != 0
                        && core.stable_subvolume_id != old.core.stable_subvolume_id)
                {
                    return Err(RestoreError::Invalid("inode version/identity"));
                }
            } else if !record.allocated.contains(&core.ino) {
                return Err(RestoreError::Invalid("unallocated inode"));
            }
        }
        let exists = |ino: Ino| self.inodes.contains_key(&ino) || imaged.contains(&ino);
        for d in &record.dentries {
            if !exists(d.dir) || d.link.is_some_and(|l| !exists(l.ino)) {
                return Err(RestoreError::Invalid("missing dentry reference"));
            }
        }
        for ino in &record.purged {
            if self.recovery_semantics == RecoverySemantics::RetainedTargets3
                && self.retains_retry_target(*ino)
            {
                return Err(RestoreError::Invalid(
                    "purge before committed reference retirement",
                ));
            }
            if !self.inodes.contains_key(ino)
                || !record
                    .lifecycle_intents
                    .iter()
                    .chain(self.jobs.values())
                    .any(|j| j.ino == *ino && matches!(j.kind, LifecycleKind::Purge { .. }))
            {
                return Err(RestoreError::Invalid("purge without intent"));
            }
        }
        for id in &record.completed_jobs {
            if !self.jobs.contains_key(id) {
                return Err(RestoreError::Invalid("missing completed job"));
            }
        }
        for (offset, j) in record.lifecycle_intents.iter().enumerate() {
            if self.next_job_id.checked_add(offset as u64) != Some(j.id) {
                return Err(RestoreError::Invalid("noncontiguous job id"));
            }
        }
        for key in record
            .completed_request
            .as_ref()
            .map(|r| SessionKey {
                client: r.client,
                incarnation: r.session_incarnation,
            })
            .into_iter()
            .chain(record.client_flush.as_ref().map(|r| SessionKey {
                client: r.client,
                incarnation: r.session_incarnation,
            }))
        {
            candidate.install_session_key(key)?;
        }
        for key in &record.closed_sessions {
            if self.cleanup_receipts.contains_key(key) {
                return Err(RestoreError::Invalid("repeated cleanup record"));
            }
            if self
                .sessions
                .get(&key.client)
                .is_some_and(|s| s.epoch != key.incarnation)
            {
                return Err(RestoreError::Invalid("closed incarnation mismatch"));
            }
        }
        candidate.apply_record(record);
        candidate.durable_seq = seq;
        candidate.submitted_seq = seq;
        for key in &record.closed_sessions {
            candidate.cleanup_receipts.insert(*key, seq);
            candidate.next_session_epoch = candidate.next_session_epoch.max(key.incarnation);
        }
        Self::validate_state(&candidate.state_image())?;
        if candidate.recovery_semantics == RecoverySemantics::RetainedTargets3
            && !candidate.retry_charge().fits(self.config.retry_limits)
        {
            return Err(RestoreError::Capacity);
        }
        self.inodes = candidate.inodes;
        self.committed_inodes = candidate.committed_inodes;
        self.sessions = candidate.sessions;
        self.ino_alloc = candidate.ino_alloc;
        self.jobs = candidate.jobs;
        self.next_job_id = candidate.next_job_id;
        self.cleanup_receipts = candidate.cleanup_receipts;
        self.submitted_seq = seq;
        self.durable_seq = seq;
        self.next_session_epoch = candidate.next_session_epoch;
        self.retry_refs = candidate.retry_refs;
        Ok(())
    }
    pub(super) fn job_layouts(&self, ino: Ino, layout: &FileLayout) -> Vec<FileLayout> {
        let mut layouts = vec![layout.clone()];
        if let Some(i) = self.inodes.get(&ino) {
            for old in &i.core.old_layouts {
                if !layouts.contains(old) {
                    layouts.reserve_exact(1);
                    layouts.push(old.clone());
                }
            }
            for pool in &i.core.old_pools {
                if !layouts.iter().any(|l| l.pool_id == *pool) {
                    let mut l = layout.clone();
                    l.pool_id = *pool;
                    layouts.reserve_exact(1);
                    layouts.push(l);
                }
            }
        }
        layouts
    }
    pub fn pending_jobs(&self) -> Vec<LifecycleIntent> {
        if self.is_failed() || !self.recovery_controls_runnable() {
            return Vec::new();
        }
        self.jobs
            .values()
            .filter(|j| self.job_is_eligible(j))
            .cloned()
            .collect()
    }
    pub(super) fn job_is_eligible(&self, j: &LifecycleIntent) -> bool {
        !self.is_failed()
            && self.recovery_controls_runnable()
            && !self
                .commits
                .iter()
                .any(|c| c.completed_jobs.contains(&j.id))
            && match j.kind {
                LifecycleKind::Purge { .. } => {
                    !self.retains_retry_target(j.ino)
                        && self.inodes.get(&j.ino).is_none_or(|i| i.caps.is_empty())
                }
                LifecycleKind::RecoverSize { .. } => {
                    self.inodes.get(&j.ino).is_some_and(|i| {
                        i.caps.values().all(|c| c.issued() & CEPH_CAP_ANY_WR == 0)
                            && !(i.core.nlink == 0
                                && i.caps.is_empty()
                                && self.purge_queue.contains(&j.ino))
                    }) && !self.jobs.values().any(|other| {
                        other.ino == j.ino && matches!(other.kind, LifecycleKind::Truncate { .. })
                    })
                }
                _ => true,
            }
    }
    /// Durable intent remains present while its completion record is pending.
    /// This is diagnostic state, not permission to start another execution.
    pub fn lifecycle_intent(&self, id: JobId) -> Option<&LifecycleIntent> {
        if self.is_failed() {
            None
        } else {
            self.jobs.get(&id)
        }
    }
    pub(super) fn emit_job(&mut self, job: &LifecycleIntent, out: &mut Vec<Effect>) {
        if self.is_failed() || !self.recovery_controls_runnable() {
            return;
        }
        if self.stepping && !self.job_is_eligible(job) {
            return;
        }
        if self.stepping {
            if let LifecycleKind::Truncate { .. } = job.kind
                && let Some(i) = self.inodes.get(&job.ino)
            {
                let clients: Vec<_> = i.caps.keys().copied().collect();
                for client in clients {
                    if !self.inodes[&job.ino].caps[&client].is_new {
                        out.push(Effect::Caps(self.cap_message(
                            job.ino,
                            client,
                            CEPH_CAP_OP_TRUNC,
                        )));
                    }
                }
            }
            out.push(Effect::Lifecycle(job.clone()));
        } else {
            match &job.kind {
                LifecycleKind::Purge { layouts, max_size } => {
                    out.push(Effect::PurgeInode(PurgeInode {
                        ino: job.ino,
                        layout: layouts[0].clone(),
                        size: *max_size,
                        is_dir: !layouts[0].is_set(),
                    }))
                }
                LifecycleKind::Truncate { .. } => self.send_truncate(job.ino, out),
                LifecycleKind::RecoverSize {
                    layouts, max_size, ..
                } => out.push(Effect::RecoverFileSize {
                    ino: job.ino,
                    layout: layouts[0].clone(),
                    max_size: *max_size,
                }),
            }
        }
    }
    pub(super) fn complete_job(
        &mut self,
        id: JobId,
        result: LifecycleResult,
        out: &mut Vec<Effect>,
    ) {
        let Some(job) = self.jobs.get(&id) else {
            return;
        };
        if !self.job_is_eligible(job) {
            return;
        }
        let mut txn = Txn::new(RecordKind::LifecycleComplete);
        let ino = job.ino;
        match (&job.kind, result) {
            (
                LifecycleKind::RecoverSize {
                    inode_version,
                    truncate_seq,
                    max_size,
                    ..
                },
                LifecycleResult::Recovered { size, mtime },
            ) => {
                let Some(i) = self.inodes.get(&job.ino) else {
                    return;
                };
                if i.core.version != *inode_version
                    || i.core.truncate_seq != *truncate_seq
                    || size > *max_size
                {
                    return;
                }
                let bound = *max_size;
                self.touch(&mut txn, ino);
                let Some(i) = self.inodes.get_mut(&ino) else {
                    return;
                };
                i.core.size = size;
                i.core.mtime = mtime;
                i.core.max_size_ever = i.core.max_size_ever.max(bound);
                i.core
                    .client_ranges
                    .retain(|client, _| i.caps.contains_key(client));
                self.update_file_rstat(&mut txn, ino);
            }
            (
                LifecycleKind::Purge { .. } | LifecycleKind::Truncate { .. },
                LifecycleResult::Done,
            ) => {}
            _ => return,
        }
        txn.rec.completed_jobs.push(id);
        self.submit(txn, None, out);
    }
}
