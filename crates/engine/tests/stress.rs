//! Randomized multi-client workload with delayed cap acknowledgements and
//! (optionally) delayed journal commits. After every step the namespace
//! statistics are recomputed from scratch and compared, cap invariants are
//! checked, and at the end every request must have been answered and a
//! replay of the journal must rebuild the same namespace.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::*;
use engine::consts::*;
use engine::*;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<T: Clone>(&mut self, v: &[T]) -> Option<T> {
        if v.is_empty() {
            None
        } else {
            Some(v[self.below(v.len() as u64) as usize].clone())
        }
    }
}

struct Sim {
    fs: Fs,
    rng: Rng,
    /// Latest REVOKE per (client, ino) not yet acknowledged.
    revokes: BTreeMap<(ClientId, Ino), CapMessage>,
    /// Requests sent and not answered yet.
    outstanding: BTreeSet<(ClientId, u64)>,
    manual: bool,
    stats: BTreeMap<&'static str, u64>,
    /// Sessions may go stale; evictions are then expected.
    stale: bool,
    evict: Vec<ClientId>,
}

const CLIENTS: [ClientId; 3] = [C1, C2, C3];
const NAMES: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

impl Sim {
    fn absorb(&mut self, eff: &[Effect]) {
        for e in eff {
            match e {
                Effect::Reply(r) => {
                    assert!(
                        self.outstanding.remove(&(r.client, r.tid)),
                        "unexpected or duplicate reply {r:?}"
                    );
                    *self
                        .stats
                        .entry(if r.result == 0 { "ok" } else { "err" })
                        .or_default() += 1;
                }
                Effect::PurgeInode(_) => *self.stats.entry("purge").or_default() += 1,
                Effect::TruncateData(_) => *self.stats.entry("trunc").or_default() += 1,
                Effect::Caps(m) if m.op == CEPH_CAP_OP_REVOKE => {
                    *self.stats.entry("revoke").or_default() += 1;
                    self.revokes.insert((m.client, m.ino), m.clone());
                }
                Effect::Caps(m) if m.op == CEPH_CAP_OP_GRANT => {
                    // A grant after a revoke supersedes it only if it carries a newer seq;
                    // the client acks with its latest view anyway.
                    if let Some(r) = self.revokes.get(&(m.client, m.ino))
                        && m.seq > r.seq
                    {
                        let mut newer = m.clone();
                        newer.op = CEPH_CAP_OP_REVOKE;
                        self.revokes.insert((m.client, m.ino), newer);
                    }
                }
                Effect::EvictClient { client } => {
                    assert!(self.stale, "no stale sessions here: evict {client}");
                    assert!(
                        self.fs.e.session(*client).unwrap().is_stale(),
                        "only stale clients are evicted"
                    );
                    self.evict.push(*client);
                }
                _ => {}
            }
        }
    }

    fn do_evictions(&mut self) {
        while let Some(c) = self.evict.pop() {
            if self.fs.e.session(c).is_none() {
                continue;
            }
            let eff = self.fs.e.close_session(c, true);
            self.outstanding.retain(|(cl, _)| *cl != c);
            self.revokes.retain(|(cl, _), _| *cl != c);
            self.absorb(&eff);
            self.fs.e.open_session(c);
            *self.stats.entry("evicted").or_default() += 1;
        }
    }

    fn send(&mut self, c: ClientId, op: Op) {
        let (tid, eff) = self.fs.send(c, op);
        self.outstanding.insert((c, tid));
        self.absorb(&eff);
        if self.outstanding.contains(&(c, tid)) {
            *self.stats.entry("waited").or_default() += 1;
        }
    }

    fn ack_one(&mut self) {
        let keys: Vec<_> = self.revokes.keys().cloned().collect();
        if let Some(k) = self.rng.pick(&keys) {
            let m = self.revokes.remove(&k).unwrap();
            if self.fs.e.cap(m.ino, m.client).is_some() {
                let eff = self.fs.ack(&m);
                self.absorb(&eff);
            }
        }
    }

    fn ack_all(&mut self) {
        while !self.revokes.is_empty() {
            self.ack_one();
        }
    }

    fn commit(&mut self) {
        if self.manual {
            let eff = self.fs.commit();
            self.absorb(&eff);
        }
    }

    /// A random existing directory (by walking from the root).
    fn random_dir(&mut self) -> Ino {
        let mut d = ROOT;
        for _ in 0..3 {
            let subdirs: Vec<Ino> = self
                .fs
                .e
                .list_dir(d)
                .into_iter()
                .filter(|(_, ino, primary)| {
                    *primary && self.fs.e.inode(*ino).is_some_and(|c| c.is_dir())
                })
                .map(|(_, ino, _)| ino)
                .collect();
            match self.rng.pick(&subdirs) {
                Some(s) if self.rng.below(3) != 0 => d = s,
                _ => break,
            }
        }
        d
    }

    fn random_file(&mut self) -> Option<Ino> {
        let files: Vec<Ino> = self
            .fs
            .e
            .inode_numbers()
            .into_iter()
            .filter(|i| self.fs.e.inode(*i).is_some_and(|c| c.is_file()))
            .collect();
        self.rng.pick(&files)
    }

    fn step(&mut self) {
        let c = self.rng.pick(&CLIENTS).unwrap();
        let name = self.rng.pick(&NAMES).unwrap();
        match self.rng.below(23) {
            0 | 1 => {
                let d = self.random_dir();
                self.send(
                    c,
                    Op::Mkdir {
                        path: FilePath::child(d, name),
                        mode: 0o755,
                        xattrs: XattrMap::new(),
                        ino: 0,
                    },
                );
            }
            2 | 3 => {
                let d = self.random_dir();
                let flags = if self.rng.below(2) == 0 {
                    CEPH_O_RDWR
                } else {
                    CEPH_O_RDONLY
                };
                self.send(
                    c,
                    Op::Create {
                        path: FilePath::child(d, name),
                        flags: flags | CEPH_O_CREAT,
                        mode: 0o644,
                        layout: LayoutOverride::default(),
                        xattrs: XattrMap::new(),
                        ino: 0,
                    },
                );
            }
            4 => {
                let d = self.random_dir();
                self.send(
                    c,
                    Op::Mknod {
                        path: FilePath::child(d, name),
                        mode: S_IFREG | 0o644,
                        rdev: 0,
                        xattrs: XattrMap::new(),
                        ino: 0,
                    },
                );
            }
            5 => {
                let d = self.random_dir();
                self.send(
                    c,
                    Op::Symlink {
                        path: FilePath::child(d, name),
                        target: b"target".to_vec(),
                        xattrs: XattrMap::new(),
                        ino: 0,
                    },
                );
            }
            6 | 7 => {
                let d = self.random_dir();
                self.send(
                    c,
                    Op::Unlink {
                        path: FilePath::child(d, name),
                    },
                );
            }
            8 => {
                let d = self.random_dir();
                self.send(
                    c,
                    Op::Rmdir {
                        path: FilePath::child(d, name),
                    },
                );
            }
            9 | 10 => {
                let (s, t) = (self.random_dir(), self.random_dir());
                let n2 = self.rng.pick(&NAMES).unwrap();
                self.send(
                    c,
                    Op::Rename {
                        dst: FilePath::child(t, n2),
                        src: FilePath::child(s, name),
                    },
                );
            }
            11 => {
                if let Some(f) = self.random_file() {
                    let d = self.random_dir();
                    self.send(
                        c,
                        Op::Link {
                            path: FilePath::child(d, name),
                            target: FilePath::ino(f),
                        },
                    );
                }
            }
            12 => {
                if let Some(f) = self.random_file() {
                    let size = self.rng.below(1 << 20);
                    self.send(
                        c,
                        Op::Setattr {
                            path: FilePath::ino(f),
                            args: SetattrArgs {
                                mask: CEPH_SETATTR_SIZE,
                                size,
                                ..Default::default()
                            },
                        },
                    );
                }
            }
            13 => {
                if let Some(f) = self.random_file() {
                    let flags =
                        [CEPH_O_RDONLY, CEPH_O_WRONLY, CEPH_O_RDWR][self.rng.below(3) as usize];
                    self.send(
                        c,
                        Op::Open {
                            path: FilePath::ino(f),
                            flags,
                            mode: 0,
                            mask: 0,
                            old_size: 0,
                        },
                    );
                }
            }
            14 => {
                let d = self.random_dir();
                self.send(
                    c,
                    Op::Lookup {
                        path: FilePath::child(d, name),
                        mask: CEPH_STAT_CAP_INODE_ALL,
                    },
                );
            }
            15 => {
                let d = self.random_dir();
                self.send(
                    c,
                    Op::Readdir {
                        path: FilePath::ino(d),
                        frag: 0,
                        max_entries: 0,
                        max_bytes: 0,
                        last_name: None,
                        offset_hash: 0,
                        flags: CEPH_READDIR_REPLY_BITFLAGS,
                    },
                );
            }
            16 => {
                if let Some(f) = self.random_file() {
                    self.send(
                        c,
                        Op::Setxattr {
                            path: FilePath::ino(f),
                            name: b"user.k".to_vec(),
                            value: vec![1, 2, 3],
                            flags: 0,
                        },
                    );
                }
            }
            17 => {
                // A cap flush of a size by a client holding write caps.
                if let Some(f) = self.random_file()
                    && let Some(cap) = self.fs.e.cap(f, c).cloned()
                    && cap.issued() & (FW | FX) != 0
                {
                    let size = self.rng.below(1 << 22);
                    let dirty = cap.issued() & (FW | FX);
                    let m = CapUpdate {
                        op: CEPH_CAP_OP_FLUSH,
                        ino: f,
                        cap_id: cap.cap_id,
                        seq: cap.last_sent,
                        issue_seq: cap.last_issue,
                        caps: cap.issued(),
                        wanted: cap.wanted(),
                        dirty,
                        flush_tid: self.rng.next() >> 1,
                        oldest_flush_tid: 0,
                        size,
                        mtime: self.fs.stamp(),
                        ctime: self.fs.stamp(),
                        ..Default::default()
                    };
                    let eff = self.fs.cap_update(c, m);
                    self.absorb(&eff);
                }
            }
            18 | 22 => {
                // A client drops one of its caps.
                let inos: Vec<Ino> = self
                    .fs
                    .e
                    .session(c)
                    .map(|s| s.caps.iter().copied().collect())
                    .unwrap_or_default();
                if let Some(ino) = self.rng.pick(&inos)
                    && ino != ROOT
                {
                    let eff = self.fs.release(c, ino);
                    self.revokes.remove(&(c, ino));
                    self.absorb(&eff);
                }
            }
            19 => {
                if self.stale && self.rng.below(6) == 0 {
                    let s = self.fs.e.session(c).unwrap();
                    let eff = if s.is_stale() {
                        self.fs.e.session_resume(c)
                    } else {
                        self.fs.e.session_stale(c)
                    };
                    // A stale client cannot ack until it resumes.
                    self.absorb(&eff);
                    *self.stats.entry("stale-toggle").or_default() += 1;
                } else {
                    self.ack_one();
                }
            }
            20 => {
                if self.rng.below(15) == 0 {
                    // The client goes away (and comes back with a new session).
                    let kill = self.rng.below(2) == 0;
                    let eff = self.fs.e.close_session(c, kill);
                    self.outstanding.retain(|(cl, _)| *cl != c);
                    self.revokes.retain(|(cl, _), _| *cl != c);
                    self.absorb(&eff);
                    self.fs.e.open_session(c);
                    *self.stats.entry("reconnect").or_default() += 1;
                } else {
                    self.ack_all();
                }
            }
            _ => self.commit(),
        }
    }

    /// Recompute dirstat, rstat and link counts from the dentries.
    fn check_namespace(&self) {
        let e = &self.fs.e;
        let mut links: BTreeMap<Ino, i32> = BTreeMap::new();
        fn rstat_of(
            e: &Engine<MemJournal>,
            ino: Ino,
            links: &mut BTreeMap<Ino, i32>,
        ) -> (i64, i64, i64) {
            let c = e.inode(ino).unwrap();
            if !c.is_dir() {
                let bytes = if c.is_symlink() {
                    c.symlink.len() as i64
                } else {
                    c.size as i64
                };
                return (bytes, 1, 0);
            }
            let (mut b, mut f, mut s) = (0, 0, 1);
            let (mut nf, mut ns) = (0, 0);
            for (_, child, primary) in e.list_dir(ino) {
                *links.entry(child).or_default() += 1;
                let cc = e.inode(child).expect("dentry points at a live inode");
                if cc.is_dir() {
                    ns += 1;
                } else {
                    nf += 1;
                }
                if primary {
                    let (cb, cf, cs) = rstat_of(e, child, links);
                    b += cb;
                    f += cf;
                    s += cs;
                }
            }
            assert_eq!(
                (c.dirstat.nfiles, c.dirstat.nsubdirs),
                (nf, ns),
                "dirstat of {ino:#x}"
            );
            assert_eq!(
                (c.rstat.rbytes, c.rstat.rfiles, c.rstat.rsubdirs),
                (b, f, s),
                "rstat of {ino:#x}"
            );
            (b, f, s)
        }
        rstat_of(e, ROOT, &mut links);
        for ino in e.inode_numbers() {
            if ino == ROOT {
                continue;
            }
            let c = e.inode(ino).unwrap();
            let n = links.get(&ino).copied().unwrap_or(0);
            if c.nlink == 0 {
                assert_eq!(n, 0, "stray {ino:#x} still linked");
                assert!(
                    !e.caps_of(ino).is_empty() || e.uncommitted_records() > 0,
                    "stray {ino:#x} without caps"
                );
            } else if c.is_dir() {
                assert_eq!((c.nlink, n), (1, 1), "dir {ino:#x}");
            } else {
                assert_eq!(c.nlink, n, "nlink of {ino:#x}");
            }
            assert!(c.rstat.rctime <= e.inode(ROOT).unwrap().rstat.rctime || c.nlink == 0);
        }
    }
}

fn run(seed: u64, steps: usize, manual: bool) {
    run_with(seed, steps, manual, false)
}

fn run_with(seed: u64, steps: usize, manual: bool, stale: bool) {
    let mut fs = if manual { Fs::manual() } else { Fs::new() };
    // Stale clients may keep conflicting caps until they are evicted.
    fs.check = !stale;
    let mut sim = Sim {
        fs,
        rng: Rng(seed),
        revokes: BTreeMap::new(),
        outstanding: BTreeSet::new(),
        manual,
        stats: BTreeMap::new(),
        stale,
        evict: Vec::new(),
    };
    for _ in 0..steps {
        sim.step();
        sim.do_evictions();
        sim.check_namespace();
    }
    // Drain: commit and acknowledge until everything is answered.
    for c in CLIENTS {
        if sim.fs.e.session(c).is_some_and(|s| s.is_stale()) {
            let eff = sim.fs.e.session_resume(c);
            sim.absorb(&eff);
        }
    }
    for _ in 0..50 {
        sim.commit();
        sim.ack_all();
        sim.do_evictions();
        if sim.outstanding.is_empty() && sim.revokes.is_empty() {
            break;
        }
    }
    assert!(
        sim.outstanding.is_empty(),
        "unanswered requests: {:?}",
        sim.outstanding
    );
    assert_eq!(sim.fs.e.waiting_requests(), 0);
    sim.check_namespace();
    sim.fs.e.check_cap_invariants().unwrap();
    // Replay rebuilds the same namespace.
    let records: Vec<_> = sim
        .fs
        .e
        .journal()
        .records()
        .iter()
        .map(|(_, r)| r.clone())
        .collect();
    let replayed = Engine::replay(config(), MemJournal::new(), records.iter());
    sim.fs.e.namespace_eq(&replayed).unwrap();
    if std::env::var("ENGINE_STRESS_VERBOSE").is_ok() {
        println!(
            "seed {seed:#x} manual {manual}: {:?}, inodes {}",
            sim.stats,
            sim.fs.e.inode_numbers().len()
        );
    }
}

/// `ENGINE_STRESS_SEEDS=<n>` scales the number of seeds (default 12).
fn seeds() -> u64 {
    std::env::var("ENGINE_STRESS_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(12)
}

/// `ENGINE_STRESS_STEPS=<n>` scales the steps per seed (default 600).
fn steps() -> usize {
    std::env::var("ENGINE_STRESS_STEPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600)
}

#[test]
fn random_workload_auto_commit() {
    for seed in 1..=seeds() {
        run(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15), steps(), false);
    }
}

#[test]
fn random_workload_delayed_commit() {
    for seed in 1..=seeds() {
        run(
            seed.wrapping_mul(0x2545_f491_4f6c_dd1d).wrapping_add(7),
            steps(),
            true,
        );
    }
}

#[test]
fn random_workload_with_stale_sessions() {
    for seed in 1..=seeds() {
        run_with(
            seed.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(3),
            steps(),
            seed % 2 == 0,
            true,
        );
    }
}
