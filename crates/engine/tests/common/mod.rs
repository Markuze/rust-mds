//! Test harness: an engine with three open sessions and helpers that send
//! requests and pick the replies out of the returned effects.

#![allow(dead_code)]

use engine::consts::*;
use engine::*;

pub const C1: ClientId = 4101;
pub const C2: ClientId = 4102;
pub const C3: ClientId = 4103;
pub const ROOT: Ino = 1;

pub struct Fs {
    pub e: Engine<MemJournal>,
    pub tid: u64,
    pub now: u32,
    /// Effects of the last call.
    pub last: Vec<Effect>,
    /// Check cap invariants after every call.
    pub check: bool,
}

pub fn config() -> EngineConfig {
    let mut c = EngineConfig::default();
    c.pool_names.insert(3, "fast".to_string());
    c.data_pools = vec![2, 3];
    c.mkfs_stamp = Timestamp::new(1000, 0);
    c
}

impl Fs {
    pub fn new() -> Fs {
        Fs::with(config(), MemJournal::new())
    }

    pub fn manual() -> Fs {
        Fs::with(config(), MemJournal::manual())
    }

    pub fn with(cfg: EngineConfig, j: MemJournal) -> Fs {
        let mut e = Engine::new(cfg, j);
        // Tests start with a durable filesystem root.
        let seq = e.journal().last_seq();
        e.journal_mut().commit_upto(seq);
        e.journal_committed();
        for c in [C1, C2, C3] {
            e.open_session(c);
        }
        Fs {
            e,
            tid: 0,
            now: 0,
            last: Vec::new(),
            check: true,
        }
    }

    pub fn stamp(&mut self) -> Timestamp {
        self.now += 1;
        Timestamp::new(2000 + self.now, 0)
    }

    fn checked(&self, effects: &[Effect]) {
        if self.check
            && let Err(e) = self.e.check_cap_invariants()
        {
            panic!("cap invariant violated: {e}\neffects: {effects:#?}");
        }
    }

    /// Send a request; returns (tid, effects).
    pub fn send_req(&mut self, client: ClientId, mut req: Request) -> (u64, Vec<Effect>) {
        if req.tid == 0 {
            self.tid += 1;
            req.tid = self.tid;
        }
        if req.stamp.is_zero() {
            req.stamp = self.stamp();
        }
        let tid = req.tid;
        let eff = self.e.handle_request(client, req);
        self.checked(&eff);
        self.last = eff.clone();
        (tid, eff)
    }

    pub fn send(&mut self, client: ClientId, op: Op) -> (u64, Vec<Effect>) {
        let req = Request::new(0, op)
            .with_flags(CEPH_MDS_FLAG_WANT_DENTRY)
            .with_owner(1000, 1000);
        self.send_req(client, req)
    }

    /// Send and get the reply, acknowledging revocations on the way like a
    /// well-behaved client (the request may wait for them).
    pub fn call(&mut self, client: ClientId, op: Op) -> Reply {
        let (tid, eff) = self.send(client, op);
        self.finish_call(client, tid, eff)
    }

    pub fn call_req(&mut self, client: ClientId, req: Request) -> Reply {
        let (tid, eff) = self.send_req(client, req);
        self.finish_call(client, tid, eff)
    }

    fn finish_call(&mut self, client: ClientId, tid: u64, eff: Vec<Effect>) -> Reply {
        if let Some(r) = reply_for(&eff, client, tid) {
            return r.clone();
        }
        let more = self.settle(&eff);
        let mut all = eff;
        all.extend(more);
        self.last = all.clone();
        reply_for(&all, client, tid)
            .unwrap_or_else(|| panic!("no reply for tid {tid}: {all:#?}"))
            .clone()
    }

    pub fn try_call(&mut self, client: ClientId, op: Op) -> Result<Reply, Errno> {
        let r = self.call(client, op);
        if r.result == 0 {
            Ok(r)
        } else {
            Err(Errno::from_code(r.result).expect("known errno"))
        }
    }

    // ------------------------------------------------------------------
    // Operation helpers

    pub fn mkdir(&mut self, c: ClientId, parent: Ino, name: &str) -> Result<Ino, Errno> {
        self.mkdir_mode(c, parent, name, 0o755)
    }

    pub fn mkdir_mode(
        &mut self,
        c: ClientId,
        parent: Ino,
        name: &str,
        mode: u32,
    ) -> Result<Ino, Errno> {
        let r = self.try_call(
            c,
            Op::Mkdir {
                path: FilePath::child(parent, name),
                mode,
                xattrs: XattrMap::new(),
                ino: 0,
            },
        )?;
        Ok(r.target.unwrap().ino)
    }

    pub fn create(
        &mut self,
        c: ClientId,
        parent: Ino,
        name: &str,
        flags: u32,
    ) -> Result<Ino, Errno> {
        let r = self.create_reply(c, parent, name, flags)?;
        Ok(r.target.unwrap().ino)
    }

    pub fn create_reply(
        &mut self,
        c: ClientId,
        parent: Ino,
        name: &str,
        flags: u32,
    ) -> Result<Reply, Errno> {
        self.try_call(
            c,
            Op::Create {
                path: FilePath::child(parent, name),
                flags: flags | CEPH_O_CREAT,
                mode: 0o644,
                layout: LayoutOverride::default(),
                xattrs: XattrMap::new(),
                ino: 0,
            },
        )
    }

    /// A regular file created with MKNOD (no open caps).
    pub fn mkfile(&mut self, c: ClientId, parent: Ino, name: &str) -> Result<Ino, Errno> {
        self.mknod(c, parent, name, S_IFREG | 0o644, 0)
    }

    pub fn mknod(
        &mut self,
        c: ClientId,
        parent: Ino,
        name: &str,
        mode: u32,
        rdev: u32,
    ) -> Result<Ino, Errno> {
        let r = self.try_call(
            c,
            Op::Mknod {
                path: FilePath::child(parent, name),
                mode,
                rdev,
                xattrs: XattrMap::new(),
                ino: 0,
            },
        )?;
        Ok(r.target.unwrap().ino)
    }

    pub fn symlink(
        &mut self,
        c: ClientId,
        parent: Ino,
        name: &str,
        target: &str,
    ) -> Result<Ino, Errno> {
        let r = self.try_call(
            c,
            Op::Symlink {
                path: FilePath::child(parent, name),
                target: target.as_bytes().to_vec(),
                xattrs: XattrMap::new(),
                ino: 0,
            },
        )?;
        Ok(r.target.unwrap().ino)
    }

    pub fn link(
        &mut self,
        c: ClientId,
        parent: Ino,
        name: &str,
        target: Ino,
    ) -> Result<Reply, Errno> {
        self.try_call(
            c,
            Op::Link {
                path: FilePath::child(parent, name),
                target: FilePath::ino(target),
            },
        )
    }

    pub fn unlink(&mut self, c: ClientId, parent: Ino, name: &str) -> Result<Reply, Errno> {
        self.try_call(
            c,
            Op::Unlink {
                path: FilePath::child(parent, name),
            },
        )
    }

    pub fn rmdir(&mut self, c: ClientId, parent: Ino, name: &str) -> Result<Reply, Errno> {
        self.try_call(
            c,
            Op::Rmdir {
                path: FilePath::child(parent, name),
            },
        )
    }

    pub fn rename(
        &mut self,
        c: ClientId,
        sp: Ino,
        sn: &str,
        dp: Ino,
        dn: &str,
    ) -> Result<Reply, Errno> {
        self.try_call(
            c,
            Op::Rename {
                dst: FilePath::child(dp, dn),
                src: FilePath::child(sp, sn),
            },
        )
    }

    pub fn lookup(&mut self, c: ClientId, parent: Ino, name: &str) -> Result<Ino, Errno> {
        let r = self.try_call(
            c,
            Op::Lookup {
                path: FilePath::child(parent, name),
                mask: CEPH_STAT_CAP_INODE,
            },
        )?;
        Ok(r.target.unwrap().ino)
    }

    pub fn getattr(&mut self, c: ClientId, ino: Ino, mask: u32) -> Result<InodeStat, Errno> {
        let r = self.try_call(
            c,
            Op::Getattr {
                path: FilePath::ino(ino),
                mask,
            },
        )?;
        Ok(r.target.unwrap())
    }

    pub fn stat(&mut self, ino: Ino) -> engine::inode::InodeCore {
        self.e.inode(ino).expect("inode exists").clone()
    }

    pub fn setattr(
        &mut self,
        c: ClientId,
        ino: Ino,
        args: SetattrArgs,
    ) -> Result<InodeStat, Errno> {
        let r = self.try_call(
            c,
            Op::Setattr {
                path: FilePath::ino(ino),
                args,
            },
        )?;
        Ok(r.target.unwrap())
    }

    pub fn open(&mut self, c: ClientId, ino: Ino, flags: u32) -> Result<InodeStat, Errno> {
        let r = self.try_call(
            c,
            Op::Open {
                path: FilePath::ino(ino),
                flags,
                mode: 0,
                mask: 0,
                old_size: 0,
            },
        )?;
        Ok(r.target.unwrap())
    }

    pub fn setxattr(
        &mut self,
        c: ClientId,
        ino: Ino,
        name: &str,
        value: &[u8],
        flags: u32,
    ) -> Result<Reply, Errno> {
        self.try_call(
            c,
            Op::Setxattr {
                path: FilePath::ino(ino),
                name: name.as_bytes().to_vec(),
                value: value.to_vec(),
                flags,
            },
        )
    }

    pub fn rmxattr(&mut self, c: ClientId, ino: Ino, name: &str) -> Result<Reply, Errno> {
        self.try_call(
            c,
            Op::Rmxattr {
                path: FilePath::ino(ino),
                name: name.as_bytes().to_vec(),
            },
        )
    }

    pub fn getvxattr(&mut self, c: ClientId, ino: Ino, name: &str) -> Result<String, Errno> {
        let r = self.try_call(
            c,
            Op::Getvxattr {
                path: FilePath::ino(ino),
                name: name.as_bytes().to_vec(),
            },
        )?;
        match r.extra {
            ReplyExtra::Vxattr(v) => Ok(String::from_utf8(v).unwrap()),
            other => panic!("unexpected extra {other:?}"),
        }
    }

    pub fn readdir(
        &mut self,
        c: ClientId,
        ino: Ino,
        max_entries: u32,
        last_name: Option<&str>,
        offset_hash: u32,
    ) -> Result<(ReaddirReply, InodeStat), Errno> {
        let r = self.try_call(
            c,
            Op::Readdir {
                path: FilePath::ino(ino),
                frag: 0,
                max_entries,
                max_bytes: 0,
                last_name: last_name.map(|s| s.as_bytes().to_vec()),
                offset_hash,
                flags: CEPH_READDIR_REPLY_BITFLAGS,
            },
        )?;
        match r.extra {
            ReplyExtra::Readdir(rd) => Ok((rd, r.target.unwrap())),
            other => panic!("unexpected extra {other:?}"),
        }
    }

    pub fn names(&mut self, c: ClientId, dir: Ino) -> Vec<String> {
        let (rd, _) = self.readdir(c, dir, 0, None, 0).unwrap();
        rd.entries
            .iter()
            .map(|e| String::from_utf8(e.name.clone()).unwrap())
            .collect()
    }

    // ------------------------------------------------------------------
    // Cap traffic

    /// Send a cap message from a client.
    pub fn cap_update(&mut self, client: ClientId, m: CapUpdate) -> Vec<Effect> {
        let eff = self.e.handle_cap_update(client, m);
        self.checked(&eff);
        self.last = eff.clone();
        eff
    }

    /// The client acknowledges a cap message (REVOKE) holding `msg.caps`.
    pub fn ack(&mut self, msg: &CapMessage) -> Vec<Effect> {
        let m = CapUpdate {
            op: CEPH_CAP_OP_UPDATE,
            ino: msg.ino,
            cap_id: msg.cap_id,
            seq: msg.seq,
            issue_seq: msg.issue_seq,
            mseq: msg.mseq,
            caps: msg.caps,
            wanted: msg.wanted,
            ..Default::default()
        };
        self.cap_update(msg.client, m)
    }

    /// Acknowledge every REVOKE in `eff` (and in the effects that follow)
    /// until none is left; returns everything produced.
    pub fn settle(&mut self, eff: &[Effect]) -> Vec<Effect> {
        let mut all = Vec::new();
        let mut todo: Vec<CapMessage> = revokes(eff);
        while let Some(m) = todo.pop() {
            let more = self.ack(&m);
            todo.extend(revokes(&more));
            all.extend(more);
        }
        all
    }

    /// Release a client's cap on an inode (`MClientCapRelease`).
    pub fn release(&mut self, client: ClientId, ino: Ino) -> Vec<Effect> {
        let cap = self.e.cap(ino, client).expect("cap").clone();
        let eff = self.e.handle_cap_release(
            client,
            &[CapReleaseItem {
                ino,
                cap_id: cap.cap_id,
                mseq: cap.mseq,
                issue_seq: cap.last_issue,
            }],
        );
        self.checked(&eff);
        self.last = eff.clone();
        eff
    }

    /// Update the wanted set of a client's cap with an up-to-date UPDATE.
    pub fn want(&mut self, client: ClientId, ino: Ino, wanted: u32) -> Vec<Effect> {
        let cap = self.e.cap(ino, client).expect("cap").clone();
        let m = CapUpdate {
            op: CEPH_CAP_OP_UPDATE,
            ino,
            cap_id: cap.cap_id,
            seq: cap.last_sent,
            issue_seq: cap.last_issue,
            mseq: cap.mseq,
            caps: cap.issued(),
            wanted,
            ..Default::default()
        };
        self.cap_update(client, m)
    }

    pub fn commit(&mut self) -> Vec<Effect> {
        self.e.journal_mut().commit_all();
        let eff = self.e.journal_committed();
        self.checked(&eff);
        self.last = eff.clone();
        eff
    }

    pub fn issued(&self, ino: Ino, client: ClientId) -> u32 {
        self.e.cap(ino, client).map(|c| c.issued()).unwrap_or(0)
    }

    pub fn pending(&self, ino: Ino, client: ClientId) -> u32 {
        self.e.cap(ino, client).map(|c| c.pending()).unwrap_or(0)
    }
}

pub fn reply_for(eff: &[Effect], client: ClientId, tid: u64) -> Option<&Reply> {
    eff.iter().find_map(|e| match e {
        Effect::Reply(r) if r.client == client && r.tid == tid => Some(r),
        _ => None,
    })
}

pub fn replies(eff: &[Effect]) -> Vec<&Reply> {
    eff.iter()
        .filter_map(|e| match e {
            Effect::Reply(r) => Some(r),
            _ => None,
        })
        .collect()
}

pub fn cap_msgs(eff: &[Effect]) -> Vec<CapMessage> {
    eff.iter()
        .filter_map(|e| match e {
            Effect::Caps(m) => Some(m.clone()),
            _ => None,
        })
        .collect()
}

pub fn caps_to(eff: &[Effect], client: ClientId) -> Vec<CapMessage> {
    cap_msgs(eff)
        .into_iter()
        .filter(|m| m.client == client)
        .collect()
}

pub fn revokes(eff: &[Effect]) -> Vec<CapMessage> {
    cap_msgs(eff)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_REVOKE)
        .collect()
}

pub fn ts(sec: u32) -> Timestamp {
    Timestamp::new(sec, 0)
}

pub const P: u32 = CEPH_CAP_PIN;
pub const AS: u32 = CEPH_CAP_AUTH_SHARED;
pub const AX: u32 = CEPH_CAP_AUTH_EXCL;
pub const LS: u32 = CEPH_CAP_LINK_SHARED;
pub const XS: u32 = CEPH_CAP_XATTR_SHARED;
pub const XX: u32 = CEPH_CAP_XATTR_EXCL;
pub const FS: u32 = CEPH_CAP_FILE_SHARED;
pub const FX: u32 = CEPH_CAP_FILE_EXCL;
pub const FC: u32 = CEPH_CAP_FILE_CACHE;
pub const FR: u32 = CEPH_CAP_FILE_RD;
pub const FW: u32 = CEPH_CAP_FILE_WR;
pub const FB: u32 = CEPH_CAP_FILE_BUFFER;
pub const FL: u32 = CEPH_CAP_FILE_LAZYIO;
