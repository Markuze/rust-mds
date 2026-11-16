//! Shard identity used by the daemon.
//!
//! Startup opens the root object only. A later shard is a separate actor
//! and a separate object name. This module does not create that object.

/// The only WAL object the daemon opens.
pub fn root_object() -> String {
    engine::shard::wal_object(0)
}

/// A non-root shard is not created while the daemon starts.
pub fn opens_later_shard_at_startup() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::shard::{Actor, Actors, ShardError, ShardId};
    use engine::{Engine, EngineConfig, MemJournal};

    fn actor(id: u32) -> Actor<MemJournal> {
        Actor {
            id: ShardId(id),
            engine: Engine::new(EngineConfig::default(), MemJournal::new()),
            budget: 4,
            used: 0,
            cache_budget: u64::MAX,
            cache_used: 0,
            serving: true,
        }
    }

    #[test]
    fn a_stopped_shard_does_not_stop_the_root_actor() {
        assert_eq!(root_object(), "rmds.wal.0.0");
        assert!(!opens_later_shard_at_startup());
        assert_eq!(engine::shard::wal_object(1), "rmds.wal.1.0");
        let mut actors = Actors::new(vec![actor(0), actor(1)]);
        actors.stop(ShardId(1));
        assert_eq!(
            actors
                .step(ShardId(1), None, None)
                .unwrap_err()
                .unwrap_err(),
            ShardError::Down(ShardId(1))
        );
        assert!(actors.step(ShardId(0), None, None).is_ok());
    }
}
