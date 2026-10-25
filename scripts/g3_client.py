#!/usr/bin/env python3
"""Persistent recovery client and exact, no-follow application-state oracle."""
import argparse
import contextlib
import errno
import fcntl
import hashlib
import io
import importlib.util
import json
import os
from pathlib import Path
import re
import stat
import sys
import time

from restart_history import (file_expected, load_manifest, record, require, save,
                             sync_path, verify_manifest)

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "devcluster"))
from mount_guard import canonical, exact_mount, external_path
from recovery_directory import open_directory, validate_directory

_SPEC = importlib.util.spec_from_file_location("g3_sequential_history", Path(__file__).with_name("crash-history.py"))
sequential = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(sequential)

CASES = ("mixed_ack_history", "exclusive_create", "rename_overwrite", "hardlink",
         "positive_symlinks", "xattrs", "truncate_grow", "unlink_open_fd",
         "dirty_write_then_fsync", "exact_namespace_data_metadata")
PAYLOAD = bytes(range(256)) * 17 + b"g3-persistent-file"


def selected_cases(profile):
    require(profile in ("native", "nfs41"), "unsupported G3 profile")
    if profile == "native":
        return CASES
    return ("nfs41_ack_history",) + tuple(case for case in CASES if case not in
                                         ("mixed_ack_history", "xattrs", "unlink_open_fd"))


def excluded_cases(profile):
    return {} if profile == "native" else {
        "xattrs": "NFSv4.1 has no user-xattr protocol support",
        "mixed_ack_history": "native17-operation history replaced by declared NFSv4.1 history",
        "unlink_open_fd": "NFS keeps this descriptor linked; not the native zero-link case"}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def pwrite_all(fd, data, offset=0):
    while data:
        written = os.pwrite(fd, data, offset)
        require(written > 0, "write made no progress")
        offset += written
        data = data[written:]


def safe_relative(name):
    return not Path(name).is_absolute() and ".." not in Path(name).parts and name not in (".",)


def compare(mount, manifest, held_fds=None, root_fd=None, profile="native"):
    """Compare lstat types before reads. Never traverse a symlink, even a valid one."""
    name = manifest["root"]
    require(Path(name).name == name and name.startswith("g3-client-"), "unsafe G3 root")
    require(manifest.get("profile") == profile, "G3 comparison profile mismatch")
    selected_cases(profile)
    expected = manifest["entries"]
    require("" in expected and expected[""]["kind"] == "dir", "missing root directory")
    require(all(safe_relative(name) for name in expected), "unsafe manifest entry")
    actual, enumerated = set(), set()
    mount_fd = open_directory(mount)
    own_root = root_fd is None
    opened_root = None

    def check_stat(relative, st):
        require(relative in expected, f"G3 unexpected entry: {relative}")
        entry = expected[relative]
        require((st.st_ino, stat.S_IMODE(st.st_mode), st.st_uid, st.st_gid) ==
                (entry["ino"], entry["mode"], entry["uid"], entry["gid"]),
                f"G3 inode/mode/ownership changed: {relative}")
        return entry

    def walk(directory_fd, relative):
        st = os.fstat(directory_fd)
        require(stat.S_ISDIR(st.st_mode), "G3 directory replaced")
        check_stat(relative, st)
        actual.add(relative)
        # Descriptor-relative traversal never follows a replaced directory or
        # symlink parent. Scandir/stat/open errors propagate without omission.
        with os.scandir(directory_fd) as entries:
            children = sorted(entry.name for entry in entries)
        enumerated.add(relative)
        for child in children:
            path = f"{relative}/{child}" if relative else child
            st = os.stat(child, dir_fd=directory_fd, follow_symlinks=False)
            entry = check_stat(path, st)
            if entry["kind"] == "dir":
                child_fd = open_directory(child, dir_fd=directory_fd)
                try:
                    validate_directory(child, child_fd, dir_fd=directory_fd)
                    walk(child_fd, path)
                finally:
                    os.close(child_fd)
            elif entry["kind"] == "symlink":
                require(stat.S_ISLNK(st.st_mode), f"symlink replaced: {path}")
                require(os.readlink(child, dir_fd=directory_fd) == entry["target"], f"symlink target changed: {path}")
                actual.add(path)
            else:
                check_file(directory_fd, child, path, entry, st)
                actual.add(path)

    def check_file(directory_fd, child, relative, entry, st):
        require(entry["kind"] == "file" and stat.S_ISREG(st.st_mode), f"file replaced: {relative}")
        require((st.st_size, st.st_nlink, st.st_mtime_ns) ==
                (entry["size"], entry["nlink"], entry["mtime_ns"]), f"file attributes changed: {relative}")
        fd = (held_fds or {}).get(relative)
        own_file = fd is None
        if own_file:
            fd = os.open(child, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory_fd)
        try:
            opened = os.fstat(fd)
            require((opened.st_dev, opened.st_ino) == (st.st_dev, st.st_ino), f"file descriptor changed: {relative}")
            # The holder must reuse its fcntl descriptor: closing another file
            # descriptor for that inode would release its POSIX lock.
            require(sha(os.pread(fd, entry["size"] + 1, 0)) == entry["sha256"], f"file data changed: {relative}")
            if profile == "native":
                attrs = {key: os.getxattr(fd, key).hex() for key in os.listxattr(fd) if key.startswith("user.")}
                require(attrs == entry["xattrs"], f"file xattrs changed: {relative}")
            else:
                require("xattrs" not in entry, "NFSv4.1 manifest must not claim observed user xattrs")
        finally:
            if own_file:
                os.close(fd)

    try:
        opened_root = open_directory(name, dir_fd=mount_fd) if own_root else root_fd
        validate_directory(name, opened_root, dir_fd=mount_fd)
        root_entry = check_stat("", os.fstat(opened_root))
        require(root_entry["kind"] == "dir", "G3 root type changed")
        walk(opened_root, "")
        require(actual == set(expected), "G3 exact namespace differs")
        require({name for name, entry in expected.items() if entry["kind"] == "dir"} <= enumerated,
                "G3 directory enumeration incomplete")
    finally:
        if own_root and opened_root is not None:
            os.close(opened_root)
        os.close(mount_fd)


def history_integer(value, lower, upper):
    # JSON booleans and floating-point numbers must not compare as saved IDs.
    return type(value) is int and lower <= value < upper


def descriptor_proof(value, profile, inode=None):
    """Validate the exact version-2 descriptor schema before comparing copies."""
    require(type(value) is dict and set(value) == {"inode", "size", "sha256", "nlink"},
            "invalid descriptor proof fields")
    require(history_integer(value["inode"], 1, 1 << 64), "invalid descriptor proof inode")
    require(history_integer(value["size"], 0, 1 << 63), "invalid descriptor proof size")
    require(type(value["sha256"]) is str and re.fullmatch(r"[0-9a-f]{64}", value["sha256"]) is not None,
            "invalid descriptor proof SHA256")
    require(type(value["nlink"]) is int and value["nlink"] == (0 if profile == "native" else 1),
            "invalid descriptor proof profile link count")
    require(inode is None or value["inode"] == inode, "descriptor proof original inode changed")
    return value


def manifest_root(manifest, profile, name):
    """Check every saved root, including snapshots superseded by later rounds."""
    require(type(manifest) is dict and set(manifest) == {"root", "profile", "entries"},
            "invalid G3 manifest fields")
    require(manifest["root"] == name and manifest["profile"] == profile, "G3 manifest root identity/profile changed")
    entries = manifest["entries"]
    require(type(entries) is dict and all(type(path) is str and safe_relative(path) for path in entries),
            "invalid G3 manifest entries")
    root = entries.get("")
    require(type(root) is dict and set(root) == {"kind", "ino", "mode", "uid", "gid"} and root["kind"] == "dir",
            "invalid G3 manifest root type/fields")
    require(history_integer(root["ino"], 1, 1 << 64) and history_integer(root["mode"], 0, 0o10000) and
            all(history_integer(root[key], 0, 1 << 32) for key in ("uid", "gid")), "invalid G3 manifest root attributes")
    return (name, root["ino"], root["kind"], profile)


def load_snapshot(history, profile="native"):
    """Accept checked ready prefixes or terminal histories, never dirty rounds."""
    entries = [json.loads(line) for line in history.read_text().splitlines()]
    require(entries and type(entries[0]) is dict and type(entries[0].get("version")) is int and
            entries[0]["version"] == 2 and entries[0].get("event") == "header", "invalid G3 client history")
    header = entries[0]
    require(set(header) == {"event", "version", "root", "label", "profile", "selected_cases", "excluded_cases"},
            "invalid G3 history header fields")
    require(header.get("profile") == profile and header.get("selected_cases") == list(selected_cases(profile))
            and header.get("excluded_cases") == excluded_cases(profile), "G3 history coverage/profile mismatch")
    name = header.get("root")
    require(type(name) is str and Path(name).name == name and name.startswith("g3-client-"), "unsafe G3 history root")
    transitions = {"new": ("prepare",), "ready": ("arm", "release", "close"), "armed": ("recover",),
                   "released": ("close",), "closed": ()}
    record_fields = {"intent": {"event", "seq", "action", "monotonic_ns"},
                     "observation": {"event", "seq", "action", "unlinked_before_write"},
                     "ack": {"event", "seq", "action", "result", "monotonic_ns"}}
    proof_fields = {"unlinked_before_write", "unlinked_fsync_ack", "unlinked_observation"}
    ack_fields = {"inode": "unlinked_inode", "size": "unlinked_size", "sha256": "unlinked_data_sha256",
                  "nlink": "unlinked_nlink"}
    seq, round, state = 0, 0, "new"
    pending, observation, saved_fd, original_inode, original_root, snapshot = (None,) * 6
    for entry in entries[1:]:
        require(type(entry) is dict and history_integer(entry.get("seq"), 0, 1 << 64) and entry["seq"] == seq,
                "out-of-order G3 client history")
        event, action = entry.get("event"), entry.get("action")
        require(type(event) is str and event in record_fields and set(entry) <= record_fields[event],
                "invalid G3 history record fields")
        if event == "intent":
            require(pending is None and type(action) is str and action in transitions[state],
                    f"invalid G3 action state: {state} -> {action}")
            pending, observation = action, None
        elif event == "observation":
            require(pending in ("arm", "recover") and action == pending and observation is None,
                    "unexpected descriptor observation")
            observation = descriptor_proof(entry.get("unlinked_before_write"), profile, original_inode)
            require(observation == saved_fd, "descriptor pre-write observation differs from saved acknowledged bytes")
        else:
            require(event == "ack" and pending is not None and action == pending, "G3 ack without intent")
            result = entry.get("result")
            require(type(result) is dict, "invalid G3 action ACK result")
            required_proofs = ({"unlinked_before_write", "unlinked_fsync_ack"} if pending == "arm" else
                               {"unlinked_before_write", "unlinked_observation"} if pending == "recover" else
                               {"unlinked_observation"} if pending == "prepare" else set())
            require(set(result) & proof_fields == required_proofs, "missing/unexpected descriptor proof in ACK")
            checked = pending in ("prepare", "recover")
            require(set(result) & set(ack_fields.values()) == (set(ack_fields.values()) if checked else set()),
                    "missing/unexpected descriptor proof ACK fields")
            require(("manifest" in result) == checked, "missing/unexpected G3 manifest in ACK")
            if pending in ("arm", "recover"):
                require(observation is not None, "missing saved descriptor proof before mutation")
                before = descriptor_proof(result["unlinked_before_write"], profile, original_inode)
                require(before == observation, "descriptor proof ACK differs from pre-write observation")
                expected_round = round + 1 if pending == "arm" else round
                require(history_integer(result.get("round"), 1, 1 << 64) and result["round"] == expected_round,
                        "invalid G3 descriptor proof ACK round")
                round = expected_round
            else:
                require("round" not in result, "unexpected G3 ACK round")
            if pending in ("prepare", "arm", "recover"):
                after = descriptor_proof(result["unlinked_fsync_ack" if pending == "arm" else "unlinked_observation"],
                                         profile, original_inode)
                data = b"unlinked-open" if pending == "prepare" else f"unlinked-open-round-{round}".encode()
                if pending == "recover":
                    data += f"/recovered-{round}".encode()
                require((after["size"], after["sha256"]) == (len(data), sha(data)),
                        "descriptor proof differs from acknowledged workload bytes/round")
                if original_inode is None:
                    original_inode = after["inode"]
                saved_fd = after
            if checked:
                flat = descriptor_proof({key: result.get(field) for key, field in ack_fields.items()}, profile, original_inode)
                require(flat == saved_fd, "descriptor proof differs from matching ACK fields")
                require(result.get("profile") == profile and result.get("excluded_cases") == excluded_cases(profile),
                        "G3 ACK coverage/profile mismatch")
                manifest = result["manifest"]
                root = manifest_root(manifest, profile, name)
                require(original_root is None or root == original_root, "G3 manifest original root identity changed")
                original_root = root
                if profile == "native":
                    require("unlinked" not in manifest["entries"], "native descriptor unexpectedly linked in manifest")
                else:
                    linked = manifest["entries"].get("unlinked")
                    require(type(linked) is dict and linked.get("kind") == "file", "missing NFSv4.1 linked descriptor")
                    linked_proof = descriptor_proof({key: linked.get("ino" if key == "inode" else key)
                                                     for key in ack_fields}, profile, original_inode)
                    require(linked_proof == saved_fd, "NFSv4.1 manifest differs from descriptor proof")
                snapshot = manifest
            if pending == "release":
                require(history_integer(result.get("released_ns"), 0, 1 << 64), "invalid G3 release ACK")
            elif pending == "close":
                require(result.get("closed") is True, "invalid G3 close ACK")
            state = {"prepare": "ready", "arm": "armed", "recover": "ready",
                     "release": "released", "close": "closed"}[pending]
            pending = None
            seq += 1
    require(pending is None and state in ("ready", "released", "closed") and snapshot is not None,
            "incomplete or unflushed G3 history")
    return snapshot


class Client:
    """One process retains unlinked descriptors, dirty data and both lock classes."""

    def __init__(self, mount, history, label):
        self.mount, self.history, self.label = mount, history, label
        self.root = mount / f"g3-client-{label}-{os.getpid()}-{time.time_ns()}"
        self.mount_fd, self.root_fd = open_directory(mount), None
        self.profile = "native" if label in ("kernel", "fuse") else "nfs41"
        self.expected, self.fds = {}, {}
        self.history_log, self.current_seq, self.current_action = None, None, None
        self.round = 0
        self.armed = False
        self.prepared = False

    def validate_mount(self):
        validate_directory(self.mount, self.mount_fd)

    def validate_root(self):
        self.validate_mount()
        require(self.root_fd is not None, "workload root is not open")
        validate_directory(self.root.name, self.root_fd, dir_fd=self.mount_fd)

    def check_mixed(self):
        mixed = self.history.parent / f"mixed-{self.label}.jsonl"
        if self.profile == "native":
            verify_manifest(Path(f"/proc/self/fd/{self.mount_fd}"), load_manifest(mixed))
        else:
            with contextlib.redirect_stdout(io.StringIO()):
                require(sequential.verify(self.mount, mixed, self.profile) == 0, "incomplete NFSv4.1 history")

    def expect_file(self, name, data):
        st = os.stat(name, dir_fd=self.root_fd, follow_symlinks=False)
        require(stat.S_ISREG(st.st_mode), f"regular file replaced: {name}")
        if name in self.expected:
            require(st.st_ino == self.expected[name]["ino"], f"retained inode changed: {name}")
        entry = file_expected(st.st_ino, data)
        entry.update(mtime_ns=st.st_mtime_ns, nlink=st.st_nlink)
        if self.profile != "native":
            entry.pop("xattrs")
        self.expected[name] = entry

    def new_file(self, name, data, retain=False):
        self.validate_root()
        fd = os.open(name, os.O_CREAT | os.O_EXCL | os.O_RDWR | os.O_NOFOLLOW, 0o600, dir_fd=self.root_fd)
        try:
            pwrite_all(fd, data)
            os.fsync(fd)
            self.expect_file(name, data)
            os.fsync(self.root_fd)
            ino = os.fstat(fd).st_ino
            if retain:
                self.fds[name], fd = fd, None
            return ino
        finally:
            if fd is not None:
                os.close(fd)

    def manifest(self):
        return {"root": self.root.name, "profile": self.profile, "entries": self.expected}

    def prepare(self):
        require(not self.prepared, "client already prepared")
        self.validate_mount()
        try:
            os.stat(self.root.name, dir_fd=self.mount_fd, follow_symlinks=False)
        except FileNotFoundError:
            pass
        else:
            raise ValueError("workload root already exists; refusing pre-write replacement")
        mixed = self.history.parent / f"mixed-{self.label}.jsonl"
        with contextlib.redirect_stdout(io.StringIO()):
            if self.profile == "native":
                record(Path(f"/proc/self/fd/{self.mount_fd}"), mixed)
            else:
                sequential.record(self.mount, mixed, profile=self.profile)
        self.validate_mount()
        os.mkdir(self.root.name, mode=0o700, dir_fd=self.mount_fd)
        self.root_fd = open_directory(self.root.name, dir_fd=self.mount_fd)
        self.validate_root()
        os.fchmod(self.root_fd, 0o700)
        os.mkdir("empty", mode=0o700, dir_fd=self.root_fd)
        for name in ("", "empty"):
            st = os.fstat(self.root_fd) if not name else os.stat(name, dir_fd=self.root_fd, follow_symlinks=False)
            self.expected[name] = {"kind": "dir", "ino": st.st_ino, "mode": 0o700,
                                   "uid": os.geteuid(), "gid": os.getegid()}
        source = self.new_file("source", PAYLOAD)
        replaced = self.new_file("target", b"overwritten inode")
        require(source != replaced, "exclusive creates reused an inode")
        os.link("source", "hard", src_dir_fd=self.root_fd, dst_dir_fd=self.root_fd, follow_symlinks=False)
        os.replace("source", "target", src_dir_fd=self.root_fd, dst_dir_fd=self.root_fd)
        self.expected.pop("source")
        self.expected.pop("target")
        if self.profile == "native":
            fd = os.open("target", os.O_RDONLY | os.O_NOFOLLOW, dir_fd=self.root_fd)
            try:
                for key, value in (("user.g3", b"\x00recovery\xff"), ("user.empty", b"")):
                    os.setxattr(fd, key, value)
                os.fsync(fd)
            finally:
                os.close(fd)
        for name in ("hard", "target"):
            self.expect_file(name, PAYLOAD)
            if self.profile == "native":
                self.expected[name]["xattrs"] = {"user.g3": b"\x00recovery\xff".hex(), "user.empty": ""}
            require(self.expected[name]["ino"] == source and self.expected[name]["nlink"] == 2,
                    "rename/hardlink identity changed")
        for name, target in (("link", "target"), ("dangling", "absent"), ("dir-link", "empty")):
            os.symlink(target, name, dir_fd=self.root_fd)
            st = os.stat(name, dir_fd=self.root_fd, follow_symlinks=False)
            self.expected[name] = {"kind": "symlink", "ino": st.st_ino, "mode": stat.S_IMODE(st.st_mode),
                                   "uid": st.st_uid, "gid": st.st_gid, "target": target}
        self.new_file("sized", PAYLOAD, retain=True)
        self.new_file("dirty", b"initial", retain=True)
        self.unlinked_ino = self.new_file("unlinked", b"unlinked-open", retain=True)
        self.unlink_open = self.label in ("kernel", "fuse")
        if self.unlink_open:
            os.unlink("unlinked", dir_fd=self.root_fd)
            self.expected.pop("unlinked")
        self.unlinked_data = b"unlinked-open"
        for kind in ("fcntl", "flock"):
            self.new_file(kind, b"lock", retain=True)
            self.lock(kind, fcntl.LOCK_EX | fcntl.LOCK_NB)
        os.fsync(self.root_fd)
        os.fsync(self.mount_fd)
        self.prepared = True
        return self.check()

    def lock(self, kind, operation):
        if kind == "fcntl":
            fcntl.lockf(self.fds[kind], operation, 0, 0, os.SEEK_SET)
        else:
            fcntl.flock(self.fds[kind], operation)

    def arm(self, round):
        require(self.prepared and not self.armed and round == self.round + 1, "invalid dirty round")
        self.check()
        before = self.check_unlinked()
        self.observe_unlinked(before)
        self.round = round
        fd = self.fds["sized"]
        os.ftruncate(fd, 7)
        os.ftruncate(fd, 8192)
        pwrite_all(fd, b"grown", 8187)
        os.fsync(fd)
        self.expect_file("sized", PAYLOAD[:7] + b"\0" * (8187 - 7) + b"grown")
        self.unlinked_data = f"unlinked-open-round-{round}".encode()
        os.ftruncate(self.fds["unlinked"], len(self.unlinked_data))
        pwrite_all(self.fds["unlinked"], self.unlinked_data)
        os.fsync(self.fds["unlinked"])
        if not self.unlink_open:
            self.expect_file("unlinked", self.unlinked_data)
        tag = f"dirty-{round}".encode()
        self.dirty_data = (bytes(range(256)) * 4096)[:-len(tag)] + tag
        os.ftruncate(self.fds["dirty"], len(self.dirty_data))
        pwrite_all(self.fds["dirty"], self.dirty_data)
        self.armed = True
        # This is a write-return ACK, NOT a durable metadata/FLUSH_ACK claim.
        return {"round": round, "dirty_write_ack_ns": time.monotonic_ns(),
                "unlinked_before_write": before, "unlinked_fsync_ack": self.check_unlinked(),
                "dirty_sha256": sha(self.dirty_data), "durability": "write-return-only"}

    def recover(self, round):
        require(self.armed and round == self.round, "recovery without matching dirty write")
        self.validate_root()
        # Prove every previously fsynced byte and the exact old size BEFORE any
        # recovery mutation. A later append must never repair lost saved data.
        before = self.check_unlinked()
        self.observe_unlinked(before)
        # Controller sends this after the old MDS exits, before starting recovery.
        # O_EXCL must reach a server. A cached read cannot establish progress.
        name = f"progress-{round:03d}"
        self.new_file(name, f"recovery-{round}".encode())
        first_success = time.monotonic_ns()
        os.fsync(self.fds["dirty"])
        self.expect_file("dirty", self.dirty_data)
        suffix = f"/recovered-{round}".encode()
        pwrite_all(self.fds["unlinked"], suffix, len(self.unlinked_data))
        self.unlinked_data += suffix
        os.fsync(self.fds["unlinked"])
        if not self.unlink_open:
            self.expect_file("unlinked", self.unlinked_data)
        self.armed = False
        result = self.check()
        result.update(round=round, unlinked_before_write=before, first_success_monotonic_ns=first_success,
                      dirty_fsync_ack_ns=time.monotonic_ns())
        return result

    def check_unlinked(self):
        st = os.fstat(self.fds["unlinked"])
        require(st.st_ino == self.unlinked_ino and st.st_nlink == (0 if self.unlink_open else 1)
                and st.st_size == len(self.unlinked_data),
                "unlinked-open descriptor identity/size/link count changed")
        require(os.pread(self.fds["unlinked"], st.st_size + 1, 0) == self.unlinked_data,
                "unlinked-open descriptor data changed")
        return {"inode": st.st_ino, "size": st.st_size, "sha256": sha(self.unlinked_data), "nlink": st.st_nlink}

    def observe_unlinked(self, observation):
        if self.history_log is not None:
            save(self.history_log, {"event": "observation", "seq": self.current_seq, "action": self.current_action,
                                    "unlinked_before_write": observation})

    def check(self):
        self.validate_root()
        self.check_mixed()
        compare(self.mount, self.manifest(), self.fds, root_fd=self.root_fd, profile=self.profile)
        observed = self.check_unlinked()
        require(os.readlink("link", dir_fd=self.root_fd) == "target", "positive symlink target changed")
        fd = os.open("link", os.O_RDONLY, dir_fd=self.root_fd)
        try:
            require(os.fstat(fd).st_ino == self.expected["target"]["ino"] and
                    os.pread(fd, len(PAYLOAD) + 1, 0) == PAYLOAD, "positive symlink no longer resolves")
        finally:
            os.close(fd)
        assertions = {case: case in selected_cases(self.profile) for case in CASES}
        assertions["nfs41_ack_history"] = self.profile == "nfs41"
        assertions["unlink_open_fd"] = self.unlink_open
        for case in ("dirty_write_then_fsync", "truncate_grow"):
            assertions[case] = self.round > 0 and not self.armed
        return {"manifest": self.manifest(), "assertions": assertions,
                "profile": self.profile, "excluded_cases": excluded_cases(self.profile),
                "locks": {kind: self.expected[kind]["ino"] for kind in ("fcntl", "flock")},
                "unlinked_inode": self.unlinked_ino, "unlinked_data_sha256": sha(self.unlinked_data),
                "unlinked_size": len(self.unlinked_data), "unlinked_nlink": observed["nlink"],
                "unlinked_observation": observed}

    def release(self):
        require(self.prepared and not self.armed, "cannot release an unflushed client")
        self.validate_root()
        for kind in ("fcntl", "flock"):
            self.lock(kind, fcntl.LOCK_UN)
        return {"released_ns": time.monotonic_ns()}

    def close(self):
        for fd in self.fds.values():
            os.close(fd)
        self.fds.clear()
        for name in ("root_fd", "mount_fd"):
            fd = getattr(self, name)
            if fd is not None:
                os.close(fd)
                setattr(self, name, None)
        return {"closed": True}

    def serve(self):
        with self.history.open("x") as log:
            self.history_log = log
            save(log, {"event": "header", "version": 2, "root": self.root.name, "label": self.label,
                       "profile": self.profile, "selected_cases": list(selected_cases(self.profile)),
                       "excluded_cases": excluded_cases(self.profile)})
            sync_path(self.history.parent, directory=True)
            seq = 0
            try:
                for line in sys.stdin:
                    request = json.loads(line)
                    action = request["action"]
                    require(request["seq"] == seq and action in ("prepare", "arm", "recover", "release", "close"),
                            "invalid client command sequence")
                    self.current_seq, self.current_action = seq, action
                    save(log, {"event": "intent", "seq": seq, "action": action, "monotonic_ns": time.monotonic_ns()})
                    result = getattr(self, action)(**request.get("args", {}))
                    save(log, {"event": "ack", "seq": seq, "action": action, "result": result,
                               "monotonic_ns": time.monotonic_ns()})
                    print(json.dumps({"seq": seq, "result": result}), flush=True)
                    seq += 1
                    if action == "close":
                        return 0
                raise ValueError("controller closed the client channel without close")
            finally:
                self.history_log = None
                self.close()


def contender(mount, history, root, kind, inode, root_inode):
    require(Path(root).name == root and root.startswith("g3-client-"), "unsafe contender root")
    mount_fd, root_fd, fd = open_directory(mount), None, None
    try:
        root_fd = open_directory(root, dir_fd=mount_fd)
        require(validate_directory(root, root_fd, dir_fd=mount_fd).st_ino == root_inode,
                "contender opened a different root inode")
        fd = os.open(kind, os.O_RDWR | os.O_NOFOLLOW, dir_fd=root_fd)
        require(os.fstat(fd).st_ino == inode, "contender opened a different lock inode")
        lock = fcntl.lockf if kind == "fcntl" else fcntl.flock
        with history.open("x") as log:
            save(log, {"event": "intent", "kind": kind, "ino": inode, "monotonic_ns": time.monotonic_ns()})
            try:
                lock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as error:
                if error.errno not in (errno.EAGAIN, errno.EACCES):
                    raise
            else:
                save(log, {"event": "early_grant", "monotonic_ns": time.monotonic_ns()})
                raise ValueError("independent contender acquired a held lock")
            save(log, {"event": "blocked", "monotonic_ns": time.monotonic_ns()})
            lock(fd, fcntl.LOCK_EX)
            save(log, {"event": "acquired", "monotonic_ns": time.monotonic_ns()})
            lock(fd, fcntl.LOCK_UN)
    finally:
        for number in (fd, root_fd, mount_fd):
            if number is not None:
                os.close(number)
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("client", "verify", "contender"))
    parser.add_argument("--mount", required=True, type=Path)
    parser.add_argument("--history", required=True, type=Path)
    parser.add_argument("--label", choices=("kernel", "fuse", "nfs-a", "nfs-b"))
    parser.add_argument("--root")
    parser.add_argument("--kind", choices=("fcntl", "flock"))
    parser.add_argument("--inode", type=int)
    parser.add_argument("--root-inode", type=int)
    parser.add_argument("--profile", choices=("native", "nfs41"), default="native")
    args = parser.parse_args()
    mount = canonical(args.mount)
    require(exact_mount(mount) is not None, "client mount absent")
    history = external_path(mount, args.history)
    if args.mode == "verify":
        compare(mount, load_snapshot(history, args.profile), profile=args.profile)
        print(json.dumps({"verified": True}))
        return 0
    if args.mode == "contender":
        require(args.kind is not None and args.inode is not None and args.root is not None and args.root_inode is not None,
                "missing contender identity")
        return contender(mount, history, args.root, args.kind, args.inode, args.root_inode)
    require(args.label is not None, "client label required")
    require(args.profile == ("native" if args.label in ("kernel", "fuse") else "nfs41"),
            "client label/profile mismatch")
    return Client(mount, history, args.label).serve()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        sys.exit(1)
