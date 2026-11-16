//! Shard actors. Each shard has its own engine, budget, and failure.
//! A stopped or exhausted shard does not stop the others.
//! An inode home is one record or one forward. A second forward is an error.
//! The parent keeps a boundary copy for lookup and getattr.
//!
//! Cross-shard link and rename inside one subvolume use prepare, then commit,
//! then drop. Each participant appends to its own stream. They do not change
//! the inode number. A different subvolume is `EXDEV` and does not prepare.
//! The daemon still opens only `rmds.wal.0.0` for the root shard.

use crate::errno::Errno;
use crate::inode::InodeCore;
use crate::journal::{DentryLink, DentryUpdate, Journal, JournalRecord, RecordKind, ShardMove};
use crate::types::{Ino, Timestamp};
use crate::{Engine, EngineInput, EngineStep, RecordPermit, StepError};

/// Object name of one shard's stream 0. Shard 0 stays `rmds.wal.0.0`.
pub fn wal_object(shard: u32) -> String {
    format!("rmds.wal.{shard}.0")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Home {
    pub shard: ShardId,
    pub forward: Option<ShardId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryCopy {
    pub ino: Ino,
    pub shard: ShardId,
    pub size: u64,
    pub nlink: i32,
    pub mtime: Timestamp,
    /// `rstat.rbytes` last copied from the child. Stays stale until
    /// [`Actors::propagate_rstat`].
    pub rbytes: i64,
}

pub struct Actor<J: Journal> {
    pub id: ShardId,
    pub engine: Engine<J>,
    /// How many client steps this shard may run.
    pub budget: usize,
    pub used: usize,
    /// How many cached bytes this shard may hold.
    pub cache_budget: u64,
    pub cache_used: u64,
    pub serving: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardError {
    Down(ShardId),
    Budget(ShardId),
    SecondForward(Ino),
    Exdev,
    NotPrepared,
    Absent(Ino),
    /// A remote link names a directory this shard does not own.
    UnresolvedLink(Ino),
}

pub struct Actors<J: Journal> {
    actors: Vec<Actor<J>>,
    homes: Vec<(Ino, Home)>,
    copies: Vec<BoundaryCopy>,
    prepared: Option<Prepared>,
}

struct Prepared {
    ino: Ino,
    src: ShardId,
    dst: ShardId,
}

impl<J: Journal> Actors<J> {
    pub fn new(actors: Vec<Actor<J>>) -> Self {
        Actors {
            actors,
            homes: Vec::new(),
            copies: Vec::new(),
            prepared: None,
        }
    }

    pub fn set_home(&mut self, ino: Ino, home: Home) -> Result<(), ShardError> {
        if home.forward.is_some() {
            // One forward only. A forward of a forward is rejected.
            if let Some((_, current)) = self.homes.iter().find(|(id, _)| *id == ino) {
                if current.forward.is_some() && current.shard != home.shard {
                    return Err(ShardError::SecondForward(ino));
                }
            }
        }
        if let Some(slot) = self.homes.iter_mut().find(|(id, _)| *id == ino) {
            slot.1 = home;
        } else {
            self.homes.push((ino, home));
        }
        Ok(())
    }

    /// The shard that serves `ino`: the home, or its single forward.
    pub fn owner(&self, ino: Ino) -> Result<ShardId, ShardError> {
        let Some((_, home)) = self.homes.iter().find(|(id, _)| *id == ino) else {
            return Ok(ShardId(0));
        };
        Ok(home.forward.unwrap_or(home.shard))
    }

    pub fn publish_boundary(&mut self, copy: BoundaryCopy) {
        if let Some(slot) = self.copies.iter_mut().find(|c| c.ino == copy.ino) {
            *slot = copy;
        } else {
            self.copies.push(copy);
        }
    }

    pub fn boundary(&self, ino: Ino) -> Option<&BoundaryCopy> {
        self.copies.iter().find(|c| c.ino == ino)
    }

    /// Stop one shard. Other shards keep serving.
    pub fn stop(&mut self, id: ShardId) {
        if let Some(actor) = self.actors.iter_mut().find(|a| a.id == id) {
            actor.serving = false;
        }
    }

    pub fn step(
        &mut self,
        id: ShardId,
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
    ) -> Result<EngineStep, Result<StepError, ShardError>> {
        let Some(actor) = self.actors.iter_mut().find(|a| a.id == id) else {
            return Err(Err(ShardError::Down(id)));
        };
        if !actor.serving || actor.engine.is_failed() {
            return Err(Err(ShardError::Down(id)));
        }
        if actor.used >= actor.budget || actor.cache_used > actor.cache_budget {
            return Err(Err(ShardError::Budget(id)));
        }
        match actor.engine.step(input, permit) {
            Ok(step) => {
                actor.used = actor.used.saturating_add(1);
                Ok(step)
            }
            Err(error) => Err(Ok(error)),
        }
    }

    /// Same-subvolume cross-shard link or rename. Different subvolumes are EXDEV.
    pub fn prepare_move(
        &mut self,
        ino: Ino,
        src: ShardId,
        dst: ShardId,
        src_subvolume: Ino,
        dst_subvolume: Ino,
    ) -> Result<(), ShardError> {
        if src_subvolume != dst_subvolume {
            return Err(ShardError::Exdev);
        }
        self.prepared = Some(Prepared { ino, src, dst });
        Ok(())
    }

    /// Publish the same inode number on the destination and one forward on the source.
    pub fn commit_move(&mut self) -> Result<Ino, ShardError> {
        let Some(prepared) = self.prepared.take() else {
            return Err(ShardError::NotPrepared);
        };
        self.set_home(
            prepared.ino,
            Home {
                shard: prepared.src,
                forward: Some(prepared.dst),
            },
        )?;
        Ok(prepared.ino)
    }

    /// Account `bytes` on one shard. A full cache does not stop another shard.
    pub fn charge_cache(&mut self, id: ShardId, bytes: u64) -> Result<(), ShardError> {
        let Some(actor) = self.actors.iter_mut().find(|a| a.id == id) else {
            return Err(ShardError::Down(id));
        };
        if !actor.serving || actor.engine.is_failed() {
            return Err(ShardError::Down(id));
        }
        if actor.cache_used.saturating_add(bytes) > actor.cache_budget {
            return Err(ShardError::Budget(id));
        }
        actor.cache_used = actor.cache_used.saturating_add(bytes);
        Ok(())
    }

    /// The served inode on its home shard, or on the one forward.
    /// This does not scan the other shards.
    pub fn load(&self, ino: Ino) -> Result<InodeCore, ShardError> {
        let id = self.owner(ino)?;
        let Some(actor) = self.actors.iter().find(|a| a.id == id) else {
            return Err(ShardError::Down(id));
        };
        if !actor.serving || actor.engine.is_failed() {
            return Err(ShardError::Down(id));
        }
        actor
            .engine
            .inode(ino)
            .filter(|_| actor.engine.is_served(ino))
            .cloned()
            .ok_or(ShardError::Absent(ino))
    }

    /// A remote link whose directory is owned by another shard blocks a split.
    pub fn remote_link_blocks_split(&self, shard: ShardId) -> Result<(), ShardError> {
        let Some(actor) = self.actors.iter().find(|a| a.id == shard) else {
            return Err(ShardError::Down(shard));
        };
        for inode in actor.engine.inodes.values() {
            for (dir, _) in &inode.remote_links {
                if self.owner(*dir)? != shard {
                    return Err(ShardError::UnresolvedLink(inode.ino()));
                }
            }
        }
        Ok(())
    }

    /// Copy `ino` onto `dst` as an incoming image. Both streams get a prepare.
    /// Lookup on `dst` does not see it until [`Self::commit_image`].
    pub fn prepare_image(
        &mut self,
        ino: Ino,
        src: ShardId,
        dst: ShardId,
        src_subvolume: Ino,
        dst_subvolume: Ino,
    ) -> Result<(), ShardError> {
        if src_subvolume != dst_subvolume {
            return Err(ShardError::Exdev);
        }
        self.require_serving(src)?;
        self.require_serving(dst)?;
        let image = self
            .actors
            .iter()
            .find(|a| a.id == src)
            .and_then(|a| a.engine.inode(ino).cloned())
            .ok_or(ShardError::Absent(ino))?;
        let bytes = image.size.max(1);
        let dst_actor = self.actors.iter().find(|a| a.id == dst).unwrap();
        if dst_actor.cache_used.saturating_add(bytes) > dst_actor.cache_budget {
            return Err(ShardError::Budget(dst));
        }
        let mv = ShardMove {
            ino,
            src: src.0,
            dst: dst.0,
        };
        let mut token = JournalRecord::new(RecordKind::ShardPrepare);
        token.shard_move = Some(mv);
        let mut copy = token.clone();
        copy.inodes.push(image);
        self.append(src, token)?;
        self.append(dst, copy)?;
        let dst_actor = self.actors.iter_mut().find(|a| a.id == dst).unwrap();
        dst_actor.cache_used = dst_actor.cache_used.saturating_add(bytes);
        self.prepared = Some(Prepared { ino, src, dst });
        Ok(())
    }

    /// Publish the prepared image and install one forward. The inode number
    /// does not change. Both commit records are appended before the forward.
    pub fn commit_image(&mut self) -> Result<Ino, ShardError> {
        let Some(prepared) = self.prepared.take() else {
            return Err(ShardError::NotPrepared);
        };
        let mv = ShardMove {
            ino: prepared.ino,
            src: prepared.src.0,
            dst: prepared.dst.0,
        };
        let mut rec = JournalRecord::new(RecordKind::ShardCommit);
        rec.shard_move = Some(mv);
        self.append(prepared.src, rec.clone())?;
        self.append(prepared.dst, rec)?;
        self.set_home(
            prepared.ino,
            Home {
                shard: prepared.src,
                forward: Some(prepared.dst),
            },
        )?;
        Ok(prepared.ino)
    }

    /// Delete the source copy after the forward is installed.
    pub fn drop_old(&mut self, ino: Ino) -> Result<(), ShardError> {
        let Some((_, home)) = self.homes.iter().find(|(id, _)| *id == ino).copied() else {
            return Err(ShardError::NotPrepared);
        };
        let Some(dst) = home.forward else {
            return Err(ShardError::NotPrepared);
        };
        let mut rec = JournalRecord::new(RecordKind::ShardDrop);
        rec.shard_move = Some(ShardMove {
            ino,
            src: home.shard.0,
            dst: dst.0,
        });
        rec.purged.push(ino);
        self.append(home.shard, rec)?;
        Ok(())
    }

    /// Finish a move that any participant committed. Drop an incoming copy
    /// when no participant committed.
    pub fn reconcile(&mut self, records: &[JournalRecord], ino: Ino) -> Result<(), ShardError> {
        let committed = records.iter().any(|r| {
            r.kind == RecordKind::ShardCommit && r.shard_move.is_some_and(|m| m.ino == ino)
        });
        if committed {
            let mv = records
                .iter()
                .find_map(|r| r.shard_move.filter(|m| m.ino == ino))
                .ok_or(ShardError::NotPrepared)?;
            if let Some(dst) = self.actors.iter_mut().find(|a| a.id == ShardId(mv.dst)) {
                dst.engine.publish_incoming(ino);
            }
            self.set_home(
                ino,
                Home {
                    shard: ShardId(mv.src),
                    forward: Some(ShardId(mv.dst)),
                },
            )?;
            return Ok(());
        }
        let prepared = records.iter().any(|r| {
            r.kind == RecordKind::ShardPrepare && r.shard_move.is_some_and(|m| m.ino == ino)
        });
        if prepared {
            for actor in &mut self.actors {
                actor.engine.discard_incoming(ino);
            }
        }
        Ok(())
    }

    /// Rename `ino` from `src_dir`/`src_name` to `dst_dir`/`dst_name`.
    /// The inode number does not change. A different subvolume is `EXDEV`
    /// and writes nothing. The destination name and the source removal are
    /// in the commit records, so neither name is published alone.
    pub fn rename_across(
        &mut self,
        ino: Ino,
        src: ShardId,
        src_dir: Ino,
        src_name: &[u8],
        dst: ShardId,
        dst_dir: Ino,
        dst_name: &[u8],
        src_subvolume: Ino,
        dst_subvolume: Ino,
    ) -> Result<Ino, ShardError> {
        self.prepare_image(ino, src, dst, src_subvolume, dst_subvolume)?;
        let Some(prepared) = self.prepared.take() else {
            return Err(ShardError::NotPrepared);
        };
        let mv = ShardMove {
            ino,
            src: src.0,
            dst: dst.0,
        };
        let mut src_rec = JournalRecord::new(RecordKind::ShardCommit);
        src_rec.shard_move = Some(mv);
        src_rec.dentries.push(DentryUpdate {
            dir: src_dir,
            name: src_name.to_vec(),
            link: None,
        });
        let mut dst_rec = JournalRecord::new(RecordKind::ShardCommit);
        dst_rec.shard_move = Some(mv);
        dst_rec.dentries.push(DentryUpdate {
            dir: dst_dir,
            name: dst_name.to_vec(),
            link: Some(DentryLink { ino, primary: true }),
        });
        self.append(prepared.src, src_rec)?;
        self.append(prepared.dst, dst_rec)?;
        self.set_home(
            ino,
            Home {
                shard: prepared.src,
                forward: Some(prepared.dst),
            },
        )?;
        self.drop_old(ino)?;
        Ok(ino)
    }

    /// Hard-link `ino` under `dst_dir`/`dst_name` without moving it.
    /// The source name stays. A different subvolume is `EXDEV`.
    pub fn link_across(
        &mut self,
        ino: Ino,
        src: ShardId,
        dst: ShardId,
        dst_dir: Ino,
        dst_name: &[u8],
        src_subvolume: Ino,
        dst_subvolume: Ino,
    ) -> Result<Ino, ShardError> {
        if src_subvolume != dst_subvolume {
            return Err(ShardError::Exdev);
        }
        self.require_serving(src)?;
        self.require_serving(dst)?;
        let mut image = self.load(ino)?;
        image.nlink = image.nlink.saturating_add(1);
        let mut src_rec = JournalRecord::new(RecordKind::Link);
        src_rec.inodes.push(image);
        let mut dst_rec = JournalRecord::new(RecordKind::Link);
        dst_rec.dentries.push(DentryUpdate {
            dir: dst_dir,
            name: dst_name.to_vec(),
            link: Some(DentryLink {
                ino,
                primary: false,
            }),
        });
        self.append(src, src_rec)?;
        self.append(dst, dst_rec)?;
        Ok(ino)
    }

    /// Copy the child inode's `rbytes` onto its boundary copy.
    /// Until this runs, the parent copy keeps the previous value, so it is
    /// at most one unpublished delta behind.
    pub fn propagate_rstat(&mut self, ino: Ino) -> Result<i64, ShardError> {
        let bytes = self.load(ino)?.rstat.rbytes;
        let Some(copy) = self.copies.iter_mut().find(|c| c.ino == ino) else {
            return Err(ShardError::Absent(ino));
        };
        copy.rbytes = bytes;
        Ok(bytes)
    }

    fn require_serving(&self, id: ShardId) -> Result<(), ShardError> {
        match self.actors.iter().find(|a| a.id == id) {
            Some(actor) if actor.serving && !actor.engine.is_failed() => Ok(()),
            _ => Err(ShardError::Down(id)),
        }
    }

    fn append(&mut self, id: ShardId, record: JournalRecord) -> Result<(), ShardError> {
        let Some(actor) = self.actors.iter_mut().find(|a| a.id == id) else {
            return Err(ShardError::Down(id));
        };
        actor
            .engine
            .append_applied(record)
            .map(|_| ())
            .map_err(|_| ShardError::Down(id))
    }
}

impl ShardError {
    pub fn errno(&self) -> Option<Errno> {
        match self {
            ShardError::Exdev => Some(Errno::EXDEV),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inode::InodeCore;
    use crate::journal::{DentryLink, DentryUpdate, JournalRecord, RecordKind};
    use crate::{Engine, EngineConfig, MemJournal};

    fn actor(id: u32, budget: usize) -> Actor<MemJournal> {
        Actor {
            id: ShardId(id),
            engine: Engine::new(EngineConfig::default(), MemJournal::new()),
            budget,
            used: 0,
            cache_budget: u64::MAX,
            cache_used: 0,
            serving: true,
        }
    }

    #[test]
    fn a_stopped_shard_does_not_stop_another() {
        let mut actors = Actors::new(vec![actor(0, 4), actor(1, 1)]);
        actors.actors[1].serving = false;
        let down = actors
            .step(ShardId(1), None, None)
            .unwrap_err()
            .unwrap_err();
        assert_eq!(down, ShardError::Down(ShardId(1)));
        assert!(actors.step(ShardId(0), None, None).is_ok());
    }

    #[test]
    fn budgets_are_independent_and_a_forward_is_one_hop() {
        let mut actors = Actors::new(vec![actor(0, 1), actor(1, 8)]);
        assert!(actors.step(ShardId(0), None, None).is_ok());
        assert_eq!(
            actors
                .step(ShardId(0), None, None)
                .unwrap_err()
                .unwrap_err(),
            ShardError::Budget(ShardId(0))
        );
        assert!(actors.step(ShardId(1), None, None).is_ok());
        actors
            .set_home(
                9,
                Home {
                    shard: ShardId(0),
                    forward: None,
                },
            )
            .unwrap();
        actors
            .set_home(
                9,
                Home {
                    shard: ShardId(0),
                    forward: Some(ShardId(1)),
                },
            )
            .unwrap();
        assert_eq!(actors.owner(9).unwrap(), ShardId(1));
        assert_eq!(
            actors
                .set_home(
                    9,
                    Home {
                        shard: ShardId(1),
                        forward: Some(ShardId(0))
                    }
                )
                .unwrap_err(),
            ShardError::SecondForward(9)
        );
    }

    #[test]
    fn boundary_copy_and_same_subvolume_commit_keep_the_inode() {
        let mut actors = Actors::new(vec![actor(0, 4), actor(1, 4)]);
        actors.publish_boundary(BoundaryCopy {
            ino: 9,
            shard: ShardId(1),
            size: 21,
            nlink: 1,
            mtime: Timestamp::new(1, 0),
            rbytes: 0,
        });
        assert_eq!(actors.boundary(9).unwrap().size, 21);
        assert_eq!(
            actors
                .prepare_move(9, ShardId(0), ShardId(1), 4, 5)
                .unwrap_err(),
            ShardError::Exdev
        );
        actors
            .prepare_move(9, ShardId(0), ShardId(1), 4, 4)
            .unwrap();
        assert_eq!(actors.commit_move().unwrap(), 9);
        assert_eq!(actors.owner(9).unwrap(), ShardId(1));
    }

    fn seed(actors: &mut Actors<MemJournal>, ino: Ino) {
        let mut created = JournalRecord::new(RecordKind::Create);
        created
            .inodes
            .push(InodeCore::new(ino, 0o100644, 0, 0, Timestamp::new(1, 0)));
        actors.actors[0].engine.append_applied(created).unwrap();
    }

    #[test]
    fn journaled_move_keeps_the_inode_and_hides_the_uncommitted_copy() {
        assert_eq!(wal_object(0), "rmds.wal.0.0");
        assert_eq!(wal_object(1), "rmds.wal.1.0");
        let mut actors = Actors::new(vec![actor(0, 4), actor(1, 4)]);
        seed(&mut actors, 9);
        let before = actors.actors[1].engine.journal().records().len();
        assert_eq!(
            actors
                .prepare_image(9, ShardId(0), ShardId(1), 4, 5)
                .unwrap_err(),
            ShardError::Exdev
        );
        assert_eq!(actors.actors[1].engine.journal().records().len(), before);
        actors
            .prepare_image(9, ShardId(0), ShardId(1), 4, 4)
            .unwrap();
        assert!(actors.actors[0].engine.is_served(9));
        assert!(!actors.actors[1].engine.is_served(9));
        assert!(actors.actors[1].engine.inode(9).is_some());
        assert!(matches!(
            actors.actors[1].engine.checkpoint(),
            Err(crate::CheckpointError::Busy)
        ));
        let prepared: Vec<JournalRecord> = actors.actors[1]
            .engine
            .journal()
            .records()
            .iter()
            .map(|(_, rec)| rec.clone())
            .collect();
        let replayed = Engine::replay(EngineConfig::default(), MemJournal::new(), prepared.iter());
        assert!(!replayed.is_served(9));
        actors.commit_image().unwrap();
        assert!(actors.actors[1].engine.is_served(9));
        assert_eq!(actors.owner(9).unwrap(), ShardId(1));
        assert_eq!(actors.load(9).unwrap().ino, 9);
        assert!(actors.actors[0].engine.inode(9).is_some());
        actors.drop_old(9).unwrap();
        assert!(!actors.actors[0].engine.is_served(9));
        assert!(actors.actors[1].engine.is_served(9));
        assert_eq!(actors.load(9).unwrap().ino, 9);
    }

    fn replayed(records: &[JournalRecord]) -> Engine<MemJournal> {
        Engine::replay(EngineConfig::default(), MemJournal::new(), records.iter())
    }

    #[test]
    fn an_uncommitted_prepare_is_discarded_and_one_commit_finishes_the_move() {
        let mut actors = Actors::new(vec![actor(0, 4), actor(1, 4)]);
        seed(&mut actors, 9);
        actors
            .prepare_image(9, ShardId(0), ShardId(1), 7, 7)
            .unwrap();
        let prepared: Vec<JournalRecord> = actors
            .actors
            .iter()
            .flat_map(|a| a.engine.journal().records().iter().map(|(_, r)| r.clone()))
            .filter(|r| r.kind == RecordKind::ShardPrepare)
            .collect();
        let mut recovered = Actors::new(vec![
            Actor {
                engine: replayed(&prepared[..1]),
                ..actor(0, 4)
            },
            Actor {
                engine: replayed(&prepared[1..]),
                ..actor(1, 4)
            },
        ]);
        assert!(!recovered.actors[1].engine.is_served(9));
        assert!(recovered.actors[1].engine.inode(9).is_some());
        recovered.reconcile(&prepared, 9).unwrap();
        assert!(recovered.actors[1].engine.inode(9).is_none());

        actors.commit_image().unwrap();
        let src: Vec<JournalRecord> = actors.actors[0]
            .engine
            .journal()
            .records()
            .iter()
            .map(|(_, r)| r.clone())
            .collect();
        let dst: Vec<JournalRecord> = actors.actors[1]
            .engine
            .journal()
            .records()
            .iter()
            .map(|(_, r)| r.clone())
            .collect();
        let mut finished = Actors::new(vec![
            Actor {
                engine: replayed(&src),
                ..actor(0, 4)
            },
            Actor {
                engine: replayed(&dst),
                ..actor(1, 4)
            },
        ]);
        let mut both = src.clone();
        both.extend(dst);
        finished.reconcile(&both, 9).unwrap();
        assert!(finished.actors[1].engine.is_served(9));
        assert_eq!(finished.owner(9).unwrap(), ShardId(1));
        assert_eq!(finished.load(9).unwrap().ino, 9);
    }

    #[test]
    fn cache_budget_load_and_a_remote_link_are_per_shard() {
        let mut actors = Actors::new(vec![actor(0, 4), actor(1, 4)]);
        actors.actors[0].cache_budget = 4;
        assert_eq!(
            actors.charge_cache(ShardId(0), 5).unwrap_err(),
            ShardError::Budget(ShardId(0))
        );
        actors.charge_cache(ShardId(1), 5).unwrap();
        assert!(actors.step(ShardId(1), None, None).is_ok());
        actors.actors[0].cache_used = 5;
        assert_eq!(
            actors
                .step(ShardId(0), None, None)
                .unwrap_err()
                .unwrap_err(),
            ShardError::Budget(ShardId(0))
        );
        seed(&mut actors, 9);
        actors
            .set_home(
                9,
                Home {
                    shard: ShardId(0),
                    forward: Some(ShardId(1)),
                },
            )
            .unwrap();
        actors.actors[1]
            .engine
            .append_applied({
                let mut rec = JournalRecord::new(RecordKind::Create);
                rec.inodes
                    .push(InodeCore::new(9, 0o100644, 0, 0, Timestamp::new(1, 0)));
                rec
            })
            .unwrap();
        assert!(actors.actors[0].engine.is_served(9));
        assert_eq!(actors.load(9).unwrap().ino, 9);
        assert!(!actors.actors[1].engine.inodes.is_empty());
        actors.actors[1].serving = false;
        assert_eq!(actors.load(9).unwrap_err(), ShardError::Down(ShardId(1)));
        actors.publish_boundary(BoundaryCopy {
            ino: 9,
            shard: ShardId(1),
            size: 21,
            nlink: 1,
            mtime: Timestamp::new(1, 0),
            rbytes: 0,
        });
        assert_eq!(actors.boundary(9).unwrap().size, 21);
        actors.actors[1].serving = true;
        actors.actors[0]
            .engine
            .inodes
            .get_mut(&9)
            .unwrap()
            .remote_links
            .insert((11, b"link".to_vec()));
        actors
            .set_home(
                11,
                Home {
                    shard: ShardId(1),
                    forward: None,
                },
            )
            .unwrap();
        assert_eq!(
            actors.remote_link_blocks_split(ShardId(0)).unwrap_err(),
            ShardError::UnresolvedLink(9)
        );
    }

    #[test]
    fn a_down_or_full_shard_does_not_journal_the_copy() {
        assert_eq!(wal_object(0), "rmds.wal.0.0");
        assert_eq!(wal_object(2), "rmds.wal.2.0");
        let mut actors = Actors::new(vec![actor(0, 8), actor(1, 8)]);
        seed(&mut actors, 9);
        actors.actors[1].serving = false;
        let before = actors.actors[0].engine.journal().records().len();
        assert_eq!(
            actors
                .prepare_image(9, ShardId(0), ShardId(1), 4, 4)
                .unwrap_err(),
            ShardError::Down(ShardId(1))
        );
        assert_eq!(actors.actors[0].engine.journal().records().len(), before);
        actors.actors[1].serving = true;
        actors.actors[1].cache_budget = 0;
        let dest_before = actors.actors[1].engine.journal().records().len();
        assert_eq!(
            actors
                .prepare_image(9, ShardId(0), ShardId(1), 4, 4)
                .unwrap_err(),
            ShardError::Budget(ShardId(1))
        );
        assert_eq!(
            actors.actors[1].engine.journal().records().len(),
            dest_before
        );
        assert_eq!(actors.drop_old(9).unwrap_err(), ShardError::NotPrepared);
        assert_eq!(ShardError::Exdev.errno(), Some(crate::Errno::EXDEV));
    }

    #[test]
    fn an_incoming_copy_stays_out_of_lookup_until_commit() {
        let mut actors = Actors::new(vec![actor(0, 8), actor(1, 8)]);
        seed(&mut actors, 9);
        actors
            .prepare_image(9, ShardId(0), ShardId(1), 3, 3)
            .unwrap();
        let mut link = JournalRecord::new(RecordKind::Link);
        link.dentries.push(DentryUpdate {
            dir: 1,
            name: b"x".to_vec(),
            link: Some(DentryLink {
                ino: 9,
                primary: false,
            }),
        });
        actors.actors[1].engine.append_applied(link).unwrap();
        assert_eq!(actors.actors[1].engine.lookup_name(1, b"x"), None);
        actors.commit_image().unwrap();
        assert_eq!(actors.actors[1].engine.lookup_name(1, b"x"), Some(9));
        assert_eq!(actors.owner(9).unwrap(), ShardId(1));
        assert_eq!(actors.load(9).unwrap().ino, 9);
        assert_eq!(
            actors
                .set_home(
                    9,
                    Home {
                        shard: ShardId(1),
                        forward: Some(ShardId(0)),
                    },
                )
                .unwrap_err(),
            ShardError::SecondForward(9)
        );
    }

    #[test]
    fn one_commit_finishes_a_move_the_source_only_prepared() {
        let mut actors = Actors::new(vec![actor(0, 8), actor(1, 8)]);
        seed(&mut actors, 9);
        actors
            .prepare_image(9, ShardId(0), ShardId(1), 8, 8)
            .unwrap();
        let src: Vec<JournalRecord> = actors.actors[0]
            .engine
            .journal()
            .records()
            .iter()
            .map(|(_, record)| record.clone())
            .collect();
        let mut dst: Vec<JournalRecord> = actors.actors[1]
            .engine
            .journal()
            .records()
            .iter()
            .map(|(_, record)| record.clone())
            .collect();
        let mut commit = JournalRecord::new(RecordKind::ShardCommit);
        commit.shard_move = Some(crate::journal::ShardMove {
            ino: 9,
            src: 0,
            dst: 1,
        });
        dst.push(commit);
        let mut recovered = Actors::new(vec![
            Actor {
                engine: Engine::replay(EngineConfig::default(), MemJournal::new(), src.iter()),
                ..actor(0, 8)
            },
            Actor {
                engine: Engine::replay(EngineConfig::default(), MemJournal::new(), dst.iter()),
                ..actor(1, 8)
            },
        ]);
        let mut both = src;
        both.extend(dst);
        recovered.reconcile(&both, 9).unwrap();
        assert!(recovered.actors[1].engine.is_served(9));
        assert_eq!(recovered.owner(9).unwrap(), ShardId(1));
    }

    fn seed_name(actors: &mut Actors<MemJournal>, ino: Ino, name: &[u8]) {
        let mut created = JournalRecord::new(RecordKind::Create);
        created
            .inodes
            .push(InodeCore::new(ino, 0o100644, 0, 0, Timestamp::new(1, 0)));
        created.dentries.push(DentryUpdate {
            dir: 1,
            name: name.to_vec(),
            link: Some(DentryLink { ino, primary: true }),
        });
        actors.actors[0].engine.append_applied(created).unwrap();
    }

    #[test]
    fn rename_and_link_across_shards_keep_the_inode_number() {
        let mut actors = Actors::new(vec![actor(0, 8), actor(1, 8)]);
        seed_name(&mut actors, 9, b"a");
        let src_before = actors.actors[0].engine.journal().records().len();
        assert_eq!(
            actors
                .rename_across(9, ShardId(0), 1, b"a", ShardId(1), 1, b"b", 4, 5)
                .unwrap_err(),
            ShardError::Exdev
        );
        assert_eq!(
            actors.actors[0].engine.journal().records().len(),
            src_before
        );
        assert_eq!(actors.actors[0].engine.lookup_name(1, b"a"), Some(9));
        assert_eq!(
            actors
                .rename_across(9, ShardId(0), 1, b"a", ShardId(1), 1, b"b", 4, 4)
                .unwrap(),
            9
        );
        assert_eq!(actors.actors[0].engine.lookup_name(1, b"a"), None);
        assert_eq!(actors.actors[1].engine.lookup_name(1, b"b"), Some(9));
        assert_eq!(actors.owner(9).unwrap(), ShardId(1));
        assert!(!actors.actors[0].engine.is_served(9));
        assert!(actors.actors[1].engine.is_served(9));

        seed_name(&mut actors, 11, b"src");
        assert_eq!(
            actors
                .link_across(11, ShardId(0), ShardId(1), 1, b"dst", 3, 9)
                .unwrap_err(),
            ShardError::Exdev
        );
        assert_eq!(
            actors
                .link_across(11, ShardId(0), ShardId(1), 1, b"dst", 3, 3)
                .unwrap(),
            11
        );
        assert_eq!(actors.actors[0].engine.lookup_name(1, b"src"), Some(11));
        assert_eq!(actors.actors[1].engine.lookup_name(1, b"dst"), Some(11));
        assert_eq!(actors.actors[0].engine.inode(11).unwrap().nlink, 2);
        assert_eq!(actors.owner(11).unwrap(), ShardId(0));
    }

    #[test]
    fn rstat_lags_by_one_unpublished_delta() {
        let mut actors = Actors::new(vec![actor(0, 4), actor(1, 4)]);
        let mut created = JournalRecord::new(RecordKind::Create);
        let mut core = InodeCore::new(9, 0o100644, 0, 0, Timestamp::new(1, 0));
        core.rstat.rbytes = 50;
        created.inodes.push(core);
        actors.actors[1].engine.append_applied(created).unwrap();
        actors
            .set_home(
                9,
                Home {
                    shard: ShardId(1),
                    forward: None,
                },
            )
            .unwrap();
        actors.publish_boundary(BoundaryCopy {
            ino: 9,
            shard: ShardId(1),
            size: 0,
            nlink: 1,
            mtime: Timestamp::new(1, 0),
            rbytes: 0,
        });
        assert_eq!(actors.boundary(9).unwrap().rbytes, 0);
        assert_eq!(actors.propagate_rstat(9).unwrap(), 50);
        assert_eq!(actors.boundary(9).unwrap().rbytes, 50);
        let mut grown = JournalRecord::new(RecordKind::Setattr);
        let mut next = actors.actors[1].engine.inode(9).unwrap().clone();
        next.rstat.rbytes = 80;
        grown.inodes.push(next);
        actors.actors[1].engine.append_applied(grown).unwrap();
        assert_eq!(actors.boundary(9).unwrap().rbytes, 50);
        assert_eq!(actors.propagate_rstat(9).unwrap(), 80);
        assert_eq!(actors.boundary(9).unwrap().rbytes, 80);
    }
}
