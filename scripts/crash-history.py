#!/usr/bin/env python3
"""External acknowledged-history oracle. Does not create or reset filesystems."""
import argparse
import errno
import json
import os
from pathlib import Path
import stat
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "devcluster"))
from mount_guard import canonical, exact_mount, external_path
from recovery_directory import open_directory, validate_directory

OPERATIONS = [("create", "source"), ("create", "target"),
              ("rename", "source", "target"), ("link", "target", "alias"),
              ("xattr", "alias"), ("unlink", "target")]


def operations(profile):
    require(profile in ("native", "nfs41"), "unsupported acknowledged-history profile")
    return [op for op in OPERATIONS if profile == "native" or op[0] != "xattr"]


def save(log, entry):
    log.write(json.dumps(entry) + "\n")
    log.flush()
    os.fsync(log.fileno())


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def record(mount, history, pause_before=None, profile="native"):
    # The controller log must live outside CephFS and outside the MDS process.
    plan = operations(profile)
    mount_fd, root_fd = open_directory(mount), None
    try:
        with history.open("x") as log:
            root = "crash-history-" + str(time.time_ns())
            header = {"event": "root", "name": root, "version": 2, "profile": profile,
                      "excluded_cases": [] if profile == "native" else ["user-xattrs"]}
            save(log, header)
            validate_directory(mount, mount_fd)
            os.mkdir(root, dir_fd=mount_fd)
            root_fd = open_directory(root, dir_fd=mount_fd)
            os.fsync(root_fd)
            os.fsync(mount_fd)
            save(log, {"event": "ack", "op": "mkdir", "ino": os.fstat(root_fd).st_ino})
            # The root FD survives the failure. A replaced pathname is rejected
            # before the next mutation, including after a paused intent.
            for seq, operation in enumerate(plan):
                save(log, {"event": "intent", "seq": seq, "op": operation})
                if seq == pause_before:
                    require(sys.stdin.readline() == "continue\n", "intent barrier cancelled")
                validate_directory(mount, mount_fd)
                validate_directory(root, root_fd, dir_fd=mount_fd)
                op, *names = operation
                if op == "create":
                    fd = os.open(names[0], os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW,
                                 0o600, dir_fd=root_fd)
                    try:
                        data = names[0].encode()
                        require(os.write(fd, data) == len(data), "short history write")
                        os.fsync(fd)
                    finally:
                        os.close(fd)
                elif op == "rename":
                    os.rename(*names, src_dir_fd=root_fd, dst_dir_fd=root_fd)
                elif op == "link":
                    os.link(*names, src_dir_fd=root_fd, dst_dir_fd=root_fd, follow_symlinks=False)
                elif op == "xattr":
                    fd = os.open(names[0], os.O_RDONLY | os.O_NOFOLLOW, dir_fd=root_fd)
                    try:
                        os.setxattr(fd, "user.crash-history", b"acknowledged")
                        os.fsync(fd)
                    finally:
                        os.close(fd)
                elif op == "unlink":
                    os.unlink(names[0], dir_fd=root_fd)
                os.fsync(root_fd)
                save(log, {"event": "ack", "seq": seq})
            save(log, {"event": "complete"})
    finally:
        if root_fd is not None:
            os.close(root_fd)
        os.close(mount_fd)


def prefix_state(count, profile="native"):
    names, attrs = {}, set()
    for op, *args in operations(profile)[:count]:
        if op == "create":
            names[args[0]] = args[0]
        elif op == "rename":
            names[args[1]] = names.pop(args[0])
        elif op == "link":
            names[args[1]] = names[args[0]]
        elif op == "xattr":
            attrs.add(names[args[0]])
        elif op == "unlink":
            del names[args[0]]
    return names, attrs


def matches_state(root, count, partial_create=None, profile="native", root_inode=None):
    names, attrs = prefix_state(count, profile)
    try:
        root_fd = open_directory(root)
    except OSError as error:
        if error.errno in (errno.ENOENT, errno.ELOOP, errno.ENOTDIR):
            return False
        raise
    try:
        validate_directory(root, root_fd)
        if root_inode is not None and os.fstat(root_fd).st_ino != root_inode:
            return False
        if set(os.listdir(root_fd)) != set(names):
            return False
        inodes = {}
        for name, object_id in names.items():
            st = os.stat(name, dir_fd=root_fd, follow_symlinks=False)
            if not stat.S_ISREG(st.st_mode):
                return False
            expected = object_id.encode()
            if st.st_size > len(expected):
                return False
            fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=root_fd)
            try:
                opened = os.fstat(fd)
                if (opened.st_dev, opened.st_ino) != (st.st_dev, st.st_ino):
                    return False
                data = os.pread(fd, len(expected) + 1, 0)
                if profile == "native":
                    try:
                        attr = os.getxattr(fd, "user.crash-history")
                    except OSError as error:
                        if error.errno != errno.ENODATA:
                            raise
                        attr = None
                    if attr != (b"acknowledged" if object_id in attrs else None):
                        return False
            finally:
                os.close(fd)
            if not (expected.startswith(data) if name == partial_create else data == expected):
                return False
            if st.st_nlink != list(names.values()).count(object_id):
                return False
            inode = (st.st_dev, st.st_ino)
            if object_id in inodes and inodes[object_id] != inode:
                return False
            if any(other != object_id and ino == inode for other, ino in inodes.items()):
                return False
            inodes[object_id] = inode
        return True
    finally:
        os.close(root_fd)


def verify(mount, history, profile="native"):
    entries = [json.loads(line) for line in history.read_text().splitlines()]
    require(entries and entries[0].get("event") == "root", "invalid history root")
    plan = operations(profile)
    require(entries[0].get("profile", "native") == profile, "acknowledged-history profile mismatch")
    version = entries[0].get("version", 0)
    # Legacy native synthetic/partial histories keep their old read-only model.
    # New workloads bind the named root inode as well as its exact contents.
    require(version in (0, 2) and (version == 2 or profile == "native"), "unsupported history version")
    if version == 2:
        require(entries[0].get("excluded_cases") == ([] if profile == "native" else ["user-xattrs"]),
                "undeclared history coverage exclusions")
    name = entries[0]["name"]
    require(name.startswith("crash-history-") and Path(name).name == name, "unsafe history root")
    root = mount / name
    acknowledged = 0
    pending = None
    mkdir_ack = False
    complete = False
    root_inode = None
    for entry in entries[1:]:
        require(not complete, "events after completion")
        if entry.get("event") == "ack" and entry.get("op") == "mkdir":
            require(not mkdir_ack and pending is None and acknowledged == 0, "invalid mkdir ack")
            if version == 2:
                root_inode = entry.get("ino")
                require(type(root_inode) is int and 0 < root_inode < 1 << 64,
                        "missing acknowledged root inode identity")
                require(set(entry) == {"event", "op", "ino"}, "invalid mkdir acknowledgement fields")
            else:
                require(entry == {"event": "ack", "op": "mkdir"}, "invalid legacy mkdir ack")
            mkdir_ack = True
        elif entry["event"] == "intent":
            require(mkdir_ack and pending is None and entry["seq"] == acknowledged,
                    "non-sequential history intent")
            require(acknowledged < len(plan) and tuple(entry["op"]) == plan[acknowledged],
                    "unsupported history operation")
            pending = acknowledged
        elif entry["event"] == "ack":
            require(pending is not None and entry.get("seq") == pending, "ack without matching intent")
            acknowledged += 1
            pending = None
        elif entry == {"event": "complete"}:
            require(mkdir_ack and acknowledged == len(plan) and pending is None, "premature completion")
            complete = True
        else:
            raise AssertionError("unsupported history event")
    if not mkdir_ack:
        require(not root.is_symlink() and (not root.exists() or (root.is_dir() and not any(root.iterdir()))),
                "invalid pending mkdir outcome")
    else:
        allowed = matches_state(root, acknowledged, profile=profile, root_inode=root_inode)
        if pending is not None:
            operation = plan[pending]
            partial_create = operation[1] if operation[0] == "create" else None
            allowed |= matches_state(root, acknowledged + 1, partial_create, profile, root_inode)
        require(allowed, "namespace/data/link/xattr state violates acknowledged or allowed pending history")
    if not complete:
        print("PARTIAL: interrupted sequential history matches an allowed outcome; full gate remains unqualified")
        return 77
    print("PASS: acknowledged history verified; this is not a full G3 gate")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("record", "verify", "gate-status"))
    parser.add_argument("--mount", type=Path)
    parser.add_argument("--history", type=Path)
    parser.add_argument("--pause-before", type=int, choices=range(len(OPERATIONS)))
    parser.add_argument("--profile", choices=("native", "nfs41"), default="native")
    args = parser.parse_args()
    if args.mode == "gate-status":
        print("DEPENDENCY: full G2/G3 integration and concurrent/under-load history coverage are not qualified.")
        return 77
    if args.mount is None or args.history is None:
        parser.error("--mount and --history are required")
    if args.pause_before is not None and args.pause_before >= len(operations(args.profile)):
        parser.error("pause index is outside the selected history profile")
    if exact_mount(args.mount) is None:
        parser.error("--mount must be a mount point")
    mount_path = canonical(args.mount)
    history_path = external_path(mount_path, args.history)
    if args.mode == "record":
        record(mount_path, history_path, args.pause_before, args.profile)
        print("RECORDED: acknowledged history; not a recovery pass")
        return 0
    return verify(mount_path, history_path, args.profile)


if __name__ == "__main__":
    sys.exit(main())
