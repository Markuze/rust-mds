#!/usr/bin/env python3
"""External G2 acknowledged-state oracle. Never resets a filesystem."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "devcluster"))
from mount_guard import canonical, exact_mount, external_path

PAYLOAD = bytes(range(256)) * 257 + b"durable-restart-end"
FUTURE_COUNT = 16
PLAN = ([{"mkdir": name} for name in ("", "left", "right", "empty", "deleted-dir")]
        + [{"create_write_fsync": name} for name in ("left/data", "right/replace", "left/keep", "deleted-file", "allocated-then-deleted")]
        + [{"setxattr": "user.recovery"}, {"setxattr": "user.empty"}, {"link": ["left/data", "right/hard"]},
           {"rename_overwrite": ["left/data", "right/replace"]}, {"remove": "deleted-file"},
           {"remove": "allocated-then-deleted"}, {"remove": "deleted-dir"}])


def require(condition, message):
    if not condition:
        raise ValueError(message)


def save(log, entry):
    log.write(json.dumps(entry, sort_keys=True) + "\n")
    log.flush()
    os.fsync(log.fileno())


def sync_path(path, directory=False):
    fd = os.open(path, os.O_RDONLY | (os.O_DIRECTORY if directory else 0))
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def write_file(path, data):
    fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    with os.fdopen(fd, "wb") as file:
        file.write(data)
        file.flush()
        os.fsync(file.fileno())
    sync_path(path.parent, directory=True)
    return path.stat().st_ino


def file_expected(ino, data):
    return {"kind": "file", "ino": ino, "mode": 0o600, "uid": os.geteuid(), "gid": os.getegid(),
            "size": len(data), "sha256": hashlib.sha256(data).hexdigest(), "nlink": 1, "xattrs": {}}


def fresh_inode(ino, allocated):
    require(0 < ino < 1 << 48 and ino not in allocated, f"inode number reused or invalid: {ino}")


def record(mount, history):
    root_name = f"restart-history-{os.getpid()}-{time.time_ns()}"
    root = mount / root_name
    expected, allocated = {}, []
    seq = 0
    with history.open("x") as log:
        sync_path(history.parent, directory=True)
        save(log, {"event": "header", "version": 1, "root": root_name, "purpose": "restart"})

        def step(operation, call):
            nonlocal seq
            require(operation == PLAN[seq], "unexpected mixed-workload operation")
            save(log, {"event": "intent", "seq": seq, "operation": operation})
            result = call()
            save(log, {"event": "ack", "seq": seq, "result": result})
            seq += 1
            return result

        for name in ("", "left", "right", "empty", "deleted-dir"):
            path = root / name

            def mkdir(path=path):
                path.mkdir(mode=0o700)
                path.chmod(0o700)
                sync_path(path, directory=True)
                sync_path(path.parent, directory=True)
                return path.stat().st_ino

            ino = step({"mkdir": name}, mkdir)
            fresh_inode(ino, allocated)
            allocated.append(ino)
            expected[name] = {"kind": "dir", "ino": ino, "mode": 0o700,
                              "uid": os.geteuid(), "gid": os.getegid()}
        for name, data in (("left/data", PAYLOAD), ("right/replace", b"overwritten-target"),
                           ("left/keep", b"retained"), ("deleted-file", b"delete-me"),
                           ("allocated-then-deleted", b"reserved-inode")):
            ino = step({"create_write_fsync": name}, lambda: write_file(root / name, data))
            fresh_inode(ino, allocated)
            allocated.append(ino)
            expected[name] = file_expected(ino, data)
        for key, value in (("user.recovery", b"\x00acknowledged\xff"), ("user.empty", b"")):
            def set_attr():
                os.setxattr(root / "left/data", key, value)
                sync_path(root / "left/data")
            step({"setxattr": key}, set_attr)
            expected["left/data"]["xattrs"][key] = value.hex()

        def link():
            os.link(root / "left/data", root / "right/hard")
            sync_path(root / "right", directory=True)
            return (root / "right/hard").stat().st_ino

        ino = step({"link": ["left/data", "right/hard"]}, link)
        require(ino == expected["left/data"]["ino"], "LINK returned a different inode")
        expected["left/data"]["nlink"] = 2
        expected["right/hard"] = dict(expected["left/data"])

        def rename():
            os.replace(root / "left/data", root / "right/replace")
            sync_path(root / "left", directory=True)
            sync_path(root / "right", directory=True)
            return (root / "right/replace").stat().st_ino

        ino = step({"rename_overwrite": ["left/data", "right/replace"]}, rename)
        require(ino == expected["left/data"]["ino"], "RENAME changed source inode")
        expected["right/replace"] = expected.pop("left/data")
        for name in ("deleted-file", "allocated-then-deleted", "deleted-dir"):
            def remove(name=name):
                path = root / name
                path.rmdir() if expected[name]["kind"] == "dir" else path.unlink()
                sync_path(root, directory=True)
            step({"remove": name}, remove)
            del expected[name]
        for name, entry in expected.items():
            if entry["kind"] == "file":
                entry["mtime_ns"] = (root / name).stat().st_mtime_ns
        manifest = {"root": root_name, "entries": expected, "allocated": allocated}
        verify_manifest(mount, manifest)
        save(log, {"event": "complete", "steps": seq, "manifest": manifest})
    print(json.dumps({"recorded": root_name, "steps": seq, "allocated": allocated}))


def load_manifest(history):
    entries = [json.loads(line) for line in history.read_text().splitlines()]
    require(entries and entries[0].get("event") == "header" and entries[0].get("version") == 1,
            "unsupported restart history")
    header = entries[0]
    require(header.get("purpose") == "restart", "wrong history purpose")
    seq, pending = 0, False
    allocated = []
    for entry in entries[1:-1]:
        require(entry.get("seq") == seq, "non-sequential restart history")
        if entry.get("event") == "intent":
            require(not pending, "second operation before ack")
            require(seq < len(PLAN) and entry.get("operation") == PLAN[seq], "wrong mixed-workload operation")
            pending = True
        elif entry.get("event") == "ack":
            require(pending, "ack without intent")
            if seq < 10:
                fresh_inode(entry["result"], allocated)
                allocated.append(entry["result"])
            elif seq in (12, 13):
                require(entry["result"] == allocated[5], "LINK/RENAME ack did not preserve source inode")
            pending = False
            seq += 1
        else:
            raise ValueError("unexpected restart history event")
    final = entries[-1]
    require(not pending and final.get("event") == "complete" and final.get("steps") == seq,
            "incomplete restart history; never a G2 pass")
    require(seq == 17, "mixed workload did not complete every required operation")
    manifest = final["manifest"]
    require(manifest["root"] == header["root"] and Path(manifest["root"]).name == manifest["root"]
            and manifest["root"].startswith("restart-history-"), "invalid restart root")
    require(len(set(manifest["allocated"])) == len(manifest["allocated"]), "duplicate allocated inode")
    require(len(manifest["allocated"]) == 10, "missing allocated/deleted inode history")
    require(manifest["allocated"] == allocated, "manifest lost acknowledged allocated inode numbers")
    require(set(manifest["entries"]) == {"", "left", "right", "empty", "left/keep", "right/replace", "right/hard"},
            "incomplete mixed-workload manifest")
    for name in manifest["entries"]:
        require(not Path(name).is_absolute() and ".." not in Path(name).parts, "unsafe manifest path")
    return manifest


def verify_manifest(mount, manifest):
    root = mount / manifest["root"]
    require(root.is_dir() and not root.is_symlink(), "acknowledged root missing or replaced")
    actual = {""}
    enumerated = set()
    expected_dirs = {name for name, entry in manifest["entries"].items() if entry["kind"] == "dir"}
    def enumeration_failed(error):
        # An unreadable directory is damage, not evidence that it is empty.
        raise error
    for directory, dirs, files in os.walk(root, followlinks=False, onerror=enumeration_failed):
        relative = str(Path(directory).relative_to(root))
        enumerated.add("" if relative == "." else relative)
        for name in dirs + files:
            path = Path(directory) / name
            require(not path.is_symlink(), f"unexpected symlink: {path}")
            actual.add(str(path.relative_to(root)))
    # os.walk suppresses DirEntry.is_dir errors without calling onerror. Seeing
    # a name in its parent is not proof that this directory was enumerated.
    require(expected_dirs <= enumerated, "directory enumeration incomplete: expected directory was not read")
    require(actual == set(manifest["entries"]), "namespace differs: acknowledged deletion/rename/directory lost")
    for name, entry in manifest["entries"].items():
        path = root / name
        st = path.stat()
        require(st.st_ino == entry["ino"], f"inode changed: {name}")
        require(stat.S_IMODE(st.st_mode) == entry["mode"] and st.st_uid == entry["uid"]
                and st.st_gid == entry["gid"], f"mode/ownership changed: {name}")
        if entry["kind"] == "dir":
            require(stat.S_ISDIR(st.st_mode), f"directory type changed: {name}")
            continue
        require(stat.S_ISREG(st.st_mode), f"file type changed: {name}")
        require(st.st_size == entry["size"] and st.st_nlink == entry["nlink"], f"size/link count changed: {name}")
        require(hashlib.sha256(path.read_bytes()).hexdigest() == entry["sha256"], f"data changed: {name}")
        require(st.st_mtime_ns == entry["mtime_ns"], f"mtime changed: {name}")
        attrs = {key: os.getxattr(path, key).hex() for key in os.listxattr(path) if key.startswith("user.")}
        require(attrs == entry["xattrs"], f"xattrs changed: {name}")


def allocate(mount, history, future_history):
    manifest = load_manifest(history)
    verify_manifest(mount, manifest)
    allocated = list(manifest["allocated"])
    root = mount / manifest["root"]
    first_ack = None
    with future_history.open("x") as log:
        sync_path(future_history.parent, directory=True)
        save(log, {"event": "header", "purpose": "future-inode-nonreuse", "root": manifest["root"]})
        for i in range(FUTURE_COUNT):
            name, data = f"left/future-{i:04d}", f"future-{i}".encode()
            save(log, {"event": "intent", "seq": i, "create_write_fsync": name})
            ino = write_file(root / name, data)
            entry = file_expected(ino, data)
            entry["mtime_ns"] = (root / name).stat().st_mtime_ns
            observed = time.monotonic_ns()
            first_ack = observed if first_ack is None else first_ack
            save(log, {"event": "ack", "seq": i, "ino": ino, "entry": entry, "monotonic_ns": observed})
            # Preserve the observed successful result even when its inode number
            # violates the oracle. Such a history never receives completion.
            fresh_inode(ino, allocated)
            allocated.append(ino)
            manifest["entries"][name] = entry
        verify_manifest(mount, manifest)
        save(log, {"event": "complete", "count": FUTURE_COUNT, "first_success_monotonic_ns": first_ack})
    print(json.dumps({"future_allocations": FUTURE_COUNT, "first_success_monotonic_ns": first_ack}))


def with_future(manifest, history):
    events = [json.loads(line) for line in history.read_text().splitlines()]
    require(events and events[0] == {"event": "header", "purpose": "future-inode-nonreuse", "root": manifest["root"]},
            "wrong future-allocation history")
    require(len(events) == FUTURE_COUNT * 2 + 2 and events[-1].get("event") == "complete"
            and events[-1].get("count") == FUTURE_COUNT, "incomplete future-allocation history")
    allocated = list(manifest["allocated"])
    for i in range(FUTURE_COUNT):
        intent, ack = events[i * 2 + 1:i * 2 + 3]
        name = f"left/future-{i:04d}"
        require(intent == {"event": "intent", "seq": i, "create_write_fsync": name}, "wrong future intent")
        require(ack.get("event") == "ack" and ack.get("seq") == i, "missing future ack")
        fresh_inode(ack["ino"], allocated)
        allocated.append(ack["ino"])
        expected = file_expected(ack["ino"], f"future-{i}".encode())
        expected["mtime_ns"] = ack["entry"]["mtime_ns"]
        require(ack["entry"] == expected, "wrong acknowledged future file state")
        manifest["entries"][name] = expected
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("record", "verify", "allocate"))
    parser.add_argument("--mount", required=True, type=Path)
    parser.add_argument("--history", required=True, type=Path)
    parser.add_argument("--future-history", type=Path)
    args = parser.parse_args()
    mount = canonical(args.mount)
    require(exact_mount(mount) is not None, "test mount is absent")
    history = external_path(mount, args.history)
    if args.mode == "record":
        record(mount, history)
    elif args.mode == "verify":
        manifest = load_manifest(history)
        if args.future_history:
            manifest = with_future(manifest, external_path(mount, args.future_history))
        verify_manifest(mount, manifest)
        print(json.dumps({"verified": True}))
    else:
        require(args.future_history is not None, "--future-history is required")
        allocate(mount, history, external_path(mount, args.future_history))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        sys.exit(1)
