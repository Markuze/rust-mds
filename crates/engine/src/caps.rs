//! The per-(inode, client) capability and the cap-bit rules.
//!
//! `Capability` follows the client-visible protocol of `src/mds/Capability.h`
//! (`issue`, `issue_norevoke`, `confirm_receipt`, `clean_revoke_from`): a cap
//! has a `cap_id` that never changes while the cap lives, a `pending` set
//! (what the MDS currently allows), an `issued` set (`pending` plus bits whose
//! revocation the client has not acknowledged yet), the client's `wanted`
//! set, `last_sent` (the `seq` of the newest cap-carrying message), and
//! `last_issue` (the `issue_seq`, set only when a reply carries the cap).
//! Revocation is two-step: the MDS shrinks `pending` and sends a REVOKE with a
//! new `seq`; the bits leave `issued` only when the client acknowledges with a
//! cap message carrying that `seq` (or a newer one) and no longer holding them.
//!
//! Which bits clients may hold together is the client-visible outcome of the
//! C++ lock state tables (`src/mds/locks.c`): readers share `Fs Fc Fr Fl`
//! (`LOCK_SYNC`), writers together with others share `Fr Fw Fl` (`LOCK_MIX`,
//! synchronous I/O), a single writer gets `Fs Fx Fc Fr Fw Fb` (`LOCK_EXCL`),
//! and during a setattr/truncate everybody keeps at most `Fc Fb`
//! (`LOCK_XLOCK`). Here that is expressed as a symmetric conflict relation
//! between bits held by different clients ([`conflicting_bits`]) plus a
//! policy in the engine that picks targets; the C++ state machines themselves
//! are not reproduced (design D16).

use std::collections::VecDeque;

use crate::consts::*;

/// `ceph_flags_to_mode` (`src/common/ceph_fs.cc`): open flags to a file mode,
/// `None` for an invalid combination (the C++ returns -1).
pub fn flags_to_mode(flags: u32) -> Option<u32> {
    if flags & CEPH_O_DIRECTORY == CEPH_O_DIRECTORY {
        return Some(CEPH_FILE_MODE_PIN);
    }
    let mut mode = match flags & CEPH_O_ACCMODE {
        CEPH_O_WRONLY => CEPH_FILE_MODE_WR,
        CEPH_O_RDONLY => CEPH_FILE_MODE_RD,
        // CEPH_O_RDWR, and O_ACCMODE "which is what the VFS does".
        _ => CEPH_FILE_MODE_RDWR,
    };
    if flags & CEPH_O_LAZY != 0 {
        mode |= CEPH_FILE_MODE_LAZY;
    }
    Some(mode)
}

/// `ceph_caps_for_mode` (`src/common/ceph_fs.cc`): the caps a client wants for
/// an open file mode.
pub fn caps_for_mode(mode: u32) -> u32 {
    let mut caps = CEPH_CAP_PIN;
    if mode & CEPH_FILE_MODE_RD != 0 {
        caps |= CEPH_CAP_FILE_SHARED | CEPH_CAP_FILE_RD | CEPH_CAP_FILE_CACHE;
    }
    if mode & CEPH_FILE_MODE_WR != 0 {
        caps |= CEPH_CAP_FILE_EXCL
            | CEPH_CAP_FILE_WR
            | CEPH_CAP_FILE_BUFFER
            | CEPH_CAP_AUTH_SHARED
            | CEPH_CAP_AUTH_EXCL
            | CEPH_CAP_XATTR_SHARED
            | CEPH_CAP_XATTR_EXCL;
    }
    if mode & CEPH_FILE_MODE_LAZY != 0 {
        caps |= CEPH_CAP_FILE_LAZYIO;
    }
    caps
}

/// Render caps like `ccap_string` (`src/mds/mdstypes.h`), e.g. `pAsxLsXsxFsxcrwb`.
pub fn cap_string(caps: u32) -> String {
    fn push_gen(g: u32, out: &mut String) {
        for (bit, ch) in [
            (CEPH_CAP_GSHARED, 's'),
            (CEPH_CAP_GEXCL, 'x'),
            (CEPH_CAP_GCACHE, 'c'),
            (CEPH_CAP_GRD, 'r'),
            (CEPH_CAP_GWR, 'w'),
            (CEPH_CAP_GBUFFER, 'b'),
            (CEPH_CAP_GWREXTEND, 'a'),
            (CEPH_CAP_GLAZYIO, 'l'),
        ] {
            if g & bit != 0 {
                out.push(ch);
            }
        }
    }
    let mut s = String::new();
    if caps & CEPH_CAP_PIN != 0 {
        s.push('p');
    }
    let a = (caps >> CEPH_CAP_SAUTH) & 3;
    if a != 0 {
        s.push('A');
        push_gen(a, &mut s);
    }
    let l = (caps >> CEPH_CAP_SLINK) & 3;
    if l != 0 {
        s.push('L');
        push_gen(l, &mut s);
    }
    let x = (caps >> CEPH_CAP_SXATTR) & 3;
    if x != 0 {
        s.push('X');
        push_gen(x, &mut s);
    }
    let f = caps >> CEPH_CAP_SFILE;
    if f != 0 {
        s.push('F');
        push_gen(f, &mut s);
    }
    if s.is_empty() {
        s.push('-');
    }
    s
}

/// Generic file-lock bits that conflict with each generic bit when held by
/// two different clients. Derived from the any-client columns of the stable
/// `filelock` states in `src/mds/locks.c` (`SYNC` = `scrl`, `MIX` = `rwl`,
/// `LOCK`/`XLOCK` = `cb`) and from `EXCL` (`sxcrwb`) being loner-only.
fn file_conflict_set(g: u32) -> u32 {
    const S: u32 = CEPH_CAP_GSHARED;
    const X: u32 = CEPH_CAP_GEXCL;
    const C: u32 = CEPH_CAP_GCACHE;
    const R: u32 = CEPH_CAP_GRD;
    const W: u32 = CEPH_CAP_GWR;
    const B: u32 = CEPH_CAP_GBUFFER;
    const A: u32 = CEPH_CAP_GWREXTEND;
    const L: u32 = CEPH_CAP_GLAZYIO;
    match g {
        S => X | W | B,
        X => S | X | C | R | W | B | A | L,
        C => X | W,
        R => X | B,
        W => X | S | C | B,
        B => X | S | R | W | L,
        A => X,
        L => X | B,
        _ => 0,
    }
}

/// The bits of `mine` that conflict with `others` (caps held by one or more
/// *other* clients on the same inode). The relation is symmetric. For the
/// two-bit locks (auth, link, xattr) `s` conflicts with `x` and `x` with both.
pub fn conflicting_bits(mine: u32, others: u32) -> u32 {
    let mut out = 0;
    // simple locks
    for shift in [CEPH_CAP_SAUTH, CEPH_CAP_SLINK, CEPH_CAP_SXATTR] {
        let m = (mine >> shift) & 3;
        let o = (others >> shift) & 3;
        if m & CEPH_CAP_GSHARED != 0 && o & CEPH_CAP_GEXCL != 0 {
            out |= CEPH_CAP_GSHARED << shift;
        }
        if m & CEPH_CAP_GEXCL != 0 && o != 0 {
            out |= CEPH_CAP_GEXCL << shift;
        }
    }
    // file lock
    let mf = (mine >> CEPH_CAP_SFILE) & 0xff;
    let of = (others >> CEPH_CAP_SFILE) & 0xff;
    let mut bit = 1;
    while bit <= 0x80 {
        if mf & bit != 0 && file_conflict_set(bit) & of != 0 {
            out |= bit << CEPH_CAP_SFILE;
        }
        bit <<= 1;
    }
    out
}

/// One remembered revocation (`Capability::revoke_info`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevokeInfo {
    /// `pending` before the revocation.
    pub before: u32,
    /// `last_sent` when the revocation was decided.
    pub seq: u32,
    /// `last_issue` when the revocation was decided.
    pub last_issue: u32,
}

/// A client's capability on one inode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub cap_id: u64,
    pending: u32,
    issued: u32,
    wanted: u32,
    /// `seq` of the newest cap-carrying message sent for this cap.
    pub last_sent: u32,
    /// `issue_seq`: `last_sent` at the last reply that carried the cap.
    pub last_issue: u32,
    /// Migration sequence (`mseq`); constant with one rank.
    pub mseq: u32,
    revokes: VecDeque<RevokeInfo>,
    /// Created by the current request and not yet sent (`STATE_NEW`).
    pub(crate) is_new: bool,
    /// Last xattr version shipped to the client (`client_xattr_version`).
    pub(crate) client_xattr_version: u64,
    /// Session cap generation this cap was last validated in (`cap_gen`).
    pub(crate) cap_gen: u32,
    /// Last advertised durable writer bound. Reconnect starts conservatively
    /// at zero; REVOKE and TRUNC cannot announce a larger bound.
    pub(crate) client_max_size: u64,
}

impl Capability {
    pub fn new(cap_id: u64, cap_gen: u32) -> Capability {
        Capability {
            cap_id,
            pending: 0,
            issued: 0,
            wanted: 0,
            last_sent: 0,
            last_issue: 0,
            mseq: 0,
            revokes: VecDeque::new(),
            is_new: true,
            client_xattr_version: 0,
            cap_gen,
            client_max_size: 0,
        }
    }

    pub fn pending(&self) -> u32 {
        self.pending
    }
    pub fn issued(&self) -> u32 {
        self.issued
    }
    pub fn wanted(&self) -> u32 {
        self.wanted
    }
    /// Bits being revoked: issued but no longer pending.
    pub fn revoking(&self) -> u32 {
        self.issued & !self.pending
    }
    pub fn set_wanted(&mut self, w: u32) {
        self.wanted = w;
    }
    pub fn revoke_history(&self) -> impl Iterator<Item = &RevokeInfo> {
        self.revokes.iter()
    }

    fn calc_issued(&mut self) {
        self.issued = self.pending;
        for r in &self.revokes {
            self.issued |= r.before;
        }
    }

    /// `Capability::issue`: set `pending` to `c`, remembering a revocation if
    /// bits are removed; always advances `last_sent`. Returns the new seq.
    pub fn issue(&mut self, c: u32) -> u32 {
        if self.pending & !c != 0 {
            self.revokes.push_back(RevokeInfo {
                before: self.pending,
                seq: self.last_sent,
                last_issue: self.last_issue,
            });
            self.pending = c;
            self.issued |= c;
        } else if !self.pending & c != 0 {
            self.pending |= c;
            self.issued |= c;
            while let Some(back) = self.revokes.back() {
                if back.before & !self.pending == 0 {
                    self.revokes.pop_back();
                } else {
                    break;
                }
            }
        }
        self.last_sent = self.last_sent.wrapping_add(1);
        self.last_sent
    }

    /// `Capability::issue_norevoke`: only adds bits (used for caps carried in
    /// a request reply); clears the "new" state and advances `last_sent`.
    pub fn issue_norevoke(&mut self, c: u32) -> u32 {
        self.pending |= c;
        self.issued |= c;
        self.is_new = false;
        self.last_sent = self.last_sent.wrapping_add(1);
        self.last_sent
    }

    /// Advance `last_sent` without changing bits (`inc_last_seq`, used for a
    /// GRANT that only carries a new `max_size`).
    pub fn inc_last_seq(&mut self) -> u32 {
        self.last_sent = self.last_sent.wrapping_add(1);
        self.last_sent
    }

    /// `Capability::confirm_receipt`: the client acknowledged messages up to
    /// `seq` and still holds `caps`. Returns the bits whose revocation is now
    /// complete.
    pub fn confirm_receipt(&mut self, seq: u32, caps: u32) -> u32 {
        let was_revoking = self.issued & !self.pending;
        if seq == self.last_sent {
            self.revokes.clear();
            self.issued = caps;
            // don't add bits
            self.pending &= caps;
            if was_revoking != 0 && self.revoking() != 0 {
                self.revokes.push_back(RevokeInfo {
                    before: self.pending,
                    seq: self.last_sent,
                    last_issue: self.last_issue,
                });
            }
        } else {
            while let Some(front) = self.revokes.front() {
                if front.seq < seq {
                    self.revokes.pop_front();
                } else {
                    break;
                }
            }
            if !self.revokes.is_empty() {
                if self.revokes[0].seq == seq {
                    self.revokes[0].before = caps;
                }
                self.calc_issued();
            } else {
                self.issued = caps | self.pending;
            }
        }
        was_revoking & !self.issued
    }

    /// `Capability::clean_revoke_from`: a release with an old `issue_seq`
    /// raced with revocations; forget those targeting the released instance.
    pub fn clean_revoke_from(&mut self, last_issue: u32) {
        let mut changed = false;
        while let Some(front) = self.revokes.front() {
            if front.last_issue <= last_issue {
                self.revokes.pop_front();
                changed = true;
            } else {
                break;
            }
        }
        if changed {
            self.calc_issued();
        }
    }

    /// `Capability::revoke`: pretend the client acknowledged everything (used
    /// for stale sessions).
    pub fn force_revoke(&mut self) {
        let seq = self.last_sent;
        let p = self.pending;
        self.confirm_receipt(seq, p);
    }

    /// Mark that a reply carried the cap: `issue_seq = last_sent`.
    pub fn set_last_issue(&mut self) {
        self.last_issue = self.last_sent;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: u32 = CEPH_CAP_PIN;
    const AS: u32 = CEPH_CAP_AUTH_SHARED;
    const AX: u32 = CEPH_CAP_AUTH_EXCL;
    const LS: u32 = CEPH_CAP_LINK_SHARED;
    const XS: u32 = CEPH_CAP_XATTR_SHARED;
    const XX: u32 = CEPH_CAP_XATTR_EXCL;
    const FS: u32 = CEPH_CAP_FILE_SHARED;
    const FX: u32 = CEPH_CAP_FILE_EXCL;
    const FC: u32 = CEPH_CAP_FILE_CACHE;
    const FR: u32 = CEPH_CAP_FILE_RD;
    const FW: u32 = CEPH_CAP_FILE_WR;
    const FB: u32 = CEPH_CAP_FILE_BUFFER;
    const FL: u32 = CEPH_CAP_FILE_LAZYIO;

    #[test]
    fn engine_invariant_checker_rejects_pending_bits_not_issued() {
        let mut e = crate::Engine::new(Default::default(), crate::MemJournal::new());
        e.open_session(1);
        let mut cap = Capability::new(1, 0);
        cap.pending = P;
        cap.issued = 0;
        e.inodes
            .get_mut(&CEPH_INO_ROOT)
            .unwrap()
            .caps
            .insert(1, cap);
        assert!(
            e.check_cap_invariants()
                .unwrap_err()
                .contains("pending not within issued")
        );
    }

    #[test]
    fn modes_and_wanted() {
        assert_eq!(flags_to_mode(CEPH_O_RDONLY), Some(CEPH_FILE_MODE_RD));
        assert_eq!(
            flags_to_mode(CEPH_O_WRONLY | CEPH_O_CREAT),
            Some(CEPH_FILE_MODE_WR)
        );
        assert_eq!(flags_to_mode(CEPH_O_RDWR), Some(CEPH_FILE_MODE_RDWR));
        assert_eq!(flags_to_mode(3), Some(CEPH_FILE_MODE_RDWR));
        assert_eq!(
            flags_to_mode(CEPH_O_DIRECTORY | CEPH_O_RDWR),
            Some(CEPH_FILE_MODE_PIN)
        );
        assert_eq!(
            flags_to_mode(CEPH_O_LAZY),
            Some(CEPH_FILE_MODE_RD | CEPH_FILE_MODE_LAZY)
        );
        assert_eq!(caps_for_mode(CEPH_FILE_MODE_PIN), P);
        assert_eq!(caps_for_mode(CEPH_FILE_MODE_RD), P | FS | FR | FC);
        assert_eq!(
            caps_for_mode(CEPH_FILE_MODE_WR),
            P | FX | FW | FB | AS | AX | XS | XX
        );
        assert_eq!(
            cap_string(caps_for_mode(CEPH_FILE_MODE_RDWR)),
            "pAsxXsxFsxcrwb"
        );
        assert_eq!(cap_string(P | AS | LS | XS | FS | FC | FR), "pAsLsXsFscr");
        assert_eq!(cap_string(0), "-");
    }

    #[test]
    fn conflict_relation_matches_lock_tables() {
        // SYNC: readers share s c r l.
        assert_eq!(conflicting_bits(FS | FC | FR | FL, FS | FC | FR | FL), 0);
        // MIX: r w l shared.
        assert_eq!(conflicting_bits(FR | FW | FL, FR | FW | FL), 0);
        // LOCK/XLOCK: c b shared.
        assert_eq!(conflicting_bits(FC | FB, FC | FB), 0);
        // EXCL is exclusive.
        assert_eq!(conflicting_bits(FX, FC), FX);
        assert_eq!(conflicting_bits(FS, FW), FS);
        assert_eq!(conflicting_bits(FW, FS), FW);
        assert_eq!(conflicting_bits(FC, FW), FC);
        assert_eq!(conflicting_bits(FR, FB), FR);
        assert_eq!(conflicting_bits(AS, AX), AS);
        assert_eq!(conflicting_bits(AX, AS), AX);
        assert_eq!(conflicting_bits(AS, AS), 0);
        assert_eq!(conflicting_bits(P, u32::MAX), 0);
        // Symmetry over every pair of file bits.
        for a in 0..8 {
            for b in 0..8 {
                let x = 1 << (a + CEPH_CAP_SFILE);
                let y = 1 << (b + CEPH_CAP_SFILE);
                assert_eq!(
                    conflicting_bits(x, y) != 0,
                    conflicting_bits(y, x) != 0,
                    "{a} {b}"
                );
            }
        }
    }

    #[test]
    fn cxx_worked_example() {
        // capabilities.md 3.2: pending=pAsLsXsFscr, last_sent=5; revoke Fscr.
        let mut c = Capability::new(1, 0);
        c.issue_norevoke(P | AS | LS | XS | FS | FC | FR);
        c.last_sent = 5;
        let seq = c.issue(P | AS | LS | XS);
        assert_eq!(seq, 6);
        assert_eq!(c.revoking(), FS | FC | FR);
        // A racing client UPDATE with seq 5 still holding everything.
        assert_eq!(c.confirm_receipt(5, P | AS | LS | XS | FS | FC | FR), 0);
        assert_eq!(c.issued(), P | AS | LS | XS | FS | FC | FR);
        // The ack with seq 6.
        assert_eq!(c.confirm_receipt(6, P | AS | LS | XS), FS | FC | FR);
        assert_eq!(c.issued(), P | AS | LS | XS);
        assert_eq!(c.pending(), P | AS | LS | XS);
        assert_eq!(c.revoking(), 0);
    }

    #[test]
    fn stale_ack_does_not_shrink_pending() {
        let mut c = Capability::new(1, 0);
        c.issue_norevoke(P | FS | FR); // seq 1
        c.issue(P | FS); // revoke Fr, seq 2
        // An ack of seq 1 dropping Fs voluntarily does not remove pending Fs.
        c.confirm_receipt(1, P | FR);
        assert_eq!(c.pending(), P | FS);
        assert!(c.issued() & FR != 0, "revocation of Fr still outstanding");
        c.confirm_receipt(2, P);
        assert_eq!(c.issued(), P);
        assert_eq!(c.pending(), P, "client dropped Fs with an up-to-date ack");
    }

    #[test]
    fn grant_after_revoke_trims_history() {
        let mut c = Capability::new(1, 0);
        c.issue_norevoke(P | FS | FR | FC);
        c.issue(P | FS); // revoke c r
        c.issue(P | FS | FR | FC); // grant back before the ack
        assert_eq!(c.revoking(), 0);
        assert_eq!(c.revoke_history().count(), 0);
    }

    #[test]
    fn clean_revoke_from_release() {
        let mut c = Capability::new(1, 0);
        c.issue_norevoke(P | FS | FR); // seq 1
        c.set_last_issue(); // issue_seq 1
        c.issue(P); // revoke, seq 2, last_issue 1
        assert_eq!(c.revoking(), FS | FR);
        c.clean_revoke_from(1);
        assert_eq!(c.revoking(), 0);
    }

    #[test]
    fn force_revoke() {
        let mut c = Capability::new(1, 0);
        c.issue_norevoke(P | FS | FR | FW);
        c.issue(P | FS);
        c.force_revoke();
        assert_eq!(c.issued(), P | FS);
    }
}
