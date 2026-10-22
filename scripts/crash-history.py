#!/usr/bin/env python3
"""External acknowledged-history oracle. Does not create or reset filesystems."""
import argparse
import errno
import json
import os
from pathlib import Path
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "devcluster"))
from mount_guard import canonical, exact_mount, external_path

OPERATIONS = [("create", "source"), ("create", "target"),
              ("rename", "source", "target"), ("link", "target", "alias"),
              ("xattr", "alias"), ("unlink", "target")]


def save(log, entry):
    log.write(json.dumps(entry) + "\n")
    log.flush()
    os.fsync(log.fileno())


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def record(mount, history):
    # The controller log must live outside CephFS and outside the MDS process.
    with history.open("x") as log:
        root = mount / ("crash-history-" + str(time.time_ns()))
        save(log, {"event": "root", "name": root.name})
        root.mkdir()
        save(log, {"event": "ack", "op": "mkdir"})
        # Log intent before each call. A crash before ack permits either outcome.
        for seq, operation in enumerate(OPERATIONS):
            save(log, {"event": "intent", "seq": seq, "op": operation})
            op, *names = operation
            paths = [root / name for name in names]
            if op == "create":
                fd = os.open(paths[0], os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
                try:
                    data = names[0].encode()
                    require(os.write(fd, data) == len(data), "short history write")
                    os.fsync(fd)
                finally:
                    os.close(fd)
            elif op == "rename":
                os.rename(*paths)
            elif op == "link":
                os.link(*paths)
            elif op == "xattr":
                os.setxattr(paths[0], "user.crash-history", b"acknowledged")
            elif op == "unlink":
                paths[0].unlink()
            save(log, {"event": "ack", "seq": seq})
        save(log, {"event": "complete"})


def prefix_state(count):
    names, attrs = {}, set()
    for op, *args in OPERATIONS[:count]:
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


def matches_state(root, count, partial_create=None):
    names, attrs = prefix_state(count)
    if root.is_symlink() or not root.is_dir() or set(p.name for p in root.iterdir()) != set(names):
        return False
    inodes = {}
    for name, object_id in names.items():
        path = root / name
        if path.is_symlink() or not path.is_file():
            return False
        expected = object_id.encode()
        if path.stat().st_size > len(expected):
            return False
        data = path.read_bytes()
        if not (expected.startswith(data) if name == partial_create else data == expected):
            return False
        st = path.stat()
        if st.st_nlink != list(names.values()).count(object_id):
            return False
        inode = (st.st_dev, st.st_ino)
        if object_id in inodes and inodes[object_id] != inode:
            return False
        if any(other != object_id and ino == inode for other, ino in inodes.items()):
            return False
        inodes[object_id] = inode
        try:
            attr = os.getxattr(path, "user.crash-history")
        except OSError as error:
            if error.errno != errno.ENODATA:
                raise
            attr = None
        if attr != (b"acknowledged" if object_id in attrs else None):
            return False
    return True


def verify(mount, history):
    entries = [json.loads(line) for line in history.read_text().splitlines()]
    require(entries and entries[0].get("event") == "root", "invalid history root")
    name = entries[0]["name"]
    require(name.startswith("crash-history-") and Path(name).name == name, "unsafe history root")
    root = mount / name
    acknowledged = 0
    pending = None
    mkdir_ack = False
    complete = False
    for entry in entries[1:]:
        require(not complete, "events after completion")
        if entry == {"event": "ack", "op": "mkdir"}:
            require(not mkdir_ack and pending is None and acknowledged == 0, "invalid mkdir ack")
            mkdir_ack = True
        elif entry["event"] == "intent":
            require(mkdir_ack and pending is None and entry["seq"] == acknowledged,
                    "non-sequential history intent")
            require(acknowledged < len(OPERATIONS) and tuple(entry["op"]) == OPERATIONS[acknowledged],
                    "unsupported history operation")
            pending = acknowledged
        elif entry["event"] == "ack":
            require(pending is not None and entry.get("seq") == pending, "ack without matching intent")
            acknowledged += 1
            pending = None
        elif entry == {"event": "complete"}:
            require(mkdir_ack and acknowledged == len(OPERATIONS) and pending is None, "premature completion")
            complete = True
        else:
            raise AssertionError("unsupported history event")
    if not mkdir_ack:
        require(not root.is_symlink() and (not root.exists() or (root.is_dir() and not any(root.iterdir()))),
                "invalid pending mkdir outcome")
    else:
        allowed = matches_state(root, acknowledged)
        if pending is not None:
            operation = OPERATIONS[pending]
            partial_create = operation[1] if operation[0] == "create" else None
            allowed |= matches_state(root, acknowledged + 1, partial_create)
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
    args = parser.parse_args()
    if args.mode == "gate-status":
        print("DEPENDENCY: full G2/G3 integration and concurrent/under-load history coverage are not qualified.")
        return 77
    if args.mount is None or args.history is None:
        parser.error("--mount and --history are required")
    if exact_mount(args.mount) is None:
        parser.error("--mount must be a mount point")
    mount_path = canonical(args.mount)
    history_path = external_path(mount_path, args.history)
    if args.mode == "record":
        record(mount_path, history_path)
        print("RECORDED: acknowledged history; not a recovery pass")
        return 0
    return verify(mount_path, history_path)


if __name__ == "__main__":
    sys.exit(main())
