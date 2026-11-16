#!/usr/bin/env python3
"""Client-visible G4 cases for a CephFS mount of the G4 daemon.

The steps follow the adoption rules: a non-empty directory can become a
subvolume, inode numbers stay put, a different subvolume is EXDEV, and
clearing a non-empty subvolume is ENOTEMPTY. The MDS does not return
EDQUOT. After `max_files=1` it pushes the quota, and the client returns
EDQUOT. Run them with G4_MOUNT set to a CephFS mount whose ceph-rmds
command line contains target-g4.
"""
import errno
import os
from pathlib import Path
import shutil


NOT_CEPH = {errno.EOPNOTSUPP, errno.ENOTSUP}

# Relative to the scratch directory created under the mount.
STEPS = (
    ("mkdir", "volumes"),
    ("mkdir", "volumes/a"),
    ("mkdir", "volumes/a/inner"),
    ("create", "volumes/a/inner/fa", 0),
    ("pin", "volumes/a/inner/fa"),
    ("mkdir", "volumes/other"),
    ("create", "volumes/other/f", 0),
    ("pin", "volumes/other/f"),
    ("link", "volumes/a/l", "volumes/other/f", 0),
    ("setxattr", "volumes/a", "ceph.dir.subvolume", "1", 0),
    ("getxattr", "volumes/a", "ceph.dir.subvolume", "1"),
    ("same", "volumes/a/inner/fa"),
    ("same", "volumes/other/f"),
    ("setxattr", "volumes/a", "ceph.dir.subvolume", "1", 0),
    ("setxattr", "volumes/a/inner", "ceph.dir.subvolume", "1", errno.EINVAL),
    ("setxattr", "volumes/a/inner/fa", "ceph.dir.subvolume", "1", errno.EINVAL),
    ("mkdir", "volumes/b"),
    ("setxattr", "volumes/b", "ceph.dir.subvolume", "1", 0),
    ("rename", "volumes/a/inner/fa", "volumes/b/fa", errno.EXDEV),
    ("link", "volumes/b/l", "volumes/a/inner/fa", errno.EXDEV),
    ("rename", "volumes/a/inner/fa", "volumes/a/fa2", 0),
    ("same", "volumes/a/fa2"),
    ("removexattr", "volumes/a", "ceph.dir.subvolume", errno.ENOTEMPTY),
    ("getxattr", "volumes/a", "ceph.dir.subvolume", "1"),
    ("mkdir", "volumes/c"),
    ("mkdir", "volumes/c/inner"),
    ("setxattr", "volumes/c/inner", "ceph.dir.subvolume", "1", 0),
    ("removexattr", "volumes/c/inner", "ceph.dir.subvolume", 0),
    ("getxattr", "volumes/c/inner", "ceph.dir.subvolume", "0"),
    ("setxattr", "volumes/c", "ceph.dir.subvolume", "1", errno.EINVAL),
    ("removexattr", "volumes/b", "ceph.dir.subvolume", 0),
    ("getxattr", "volumes/b", "ceph.dir.subvolume", "0"),
    ("create", "volumes/b/after", 0),
    ("mkdir", "volumes/q"),
    ("setxattr", "volumes/q", "ceph.quota.max_files", "-1", errno.EINVAL),
    ("setxattr", "volumes/q", "ceph.quota.max_files", "lots", errno.EINVAL),
    ("setxattr", "volumes/q", "ceph.quota.max_files", "1", 0),
    ("getxattr", "volumes/q", "ceph.quota.max_files", "1"),
    # The MDS answers success and pushes the quota. The client returns EDQUOT.
    ("create", "volumes/q/over", errno.EDQUOT),
)


class NotCephfs(Exception):
    """The path is not a CephFS mount, so ceph.* xattrs are not the MDS."""


class WrongDaemon(Exception):
    """The active ceph-rmds is not the G4 binary."""


def mount_fstype(path):
    """The longest /proc/mounts entry that covers path, or None."""
    real = os.path.realpath(path)
    best = None
    with open("/proc/mounts", encoding="utf-8") as mounts:
        for line in mounts:
            parts = line.split()
            if len(parts) < 3:
                continue
            point = parts[1].replace("\\040", " ")
            if real == point or real.startswith(point.rstrip("/") + "/"):
                if best is None or len(point) > len(best[0]):
                    best = (point, parts[2])
    return None if best is None else best[1]


def mds_cmdlines():
    """Command lines of running ceph-rmds processes."""
    found = []
    for entry in os.listdir("/proc"):
        if not entry.isdigit():
            continue
        try:
            raw = Path(f"/proc/{entry}/cmdline").read_bytes()
        except OSError:
            continue
        text = raw.replace(b"\0", b" ").decode(errors="replace")
        if "ceph-rmds" in text:
            found.append(text)
    return found


def require_g4_daemon(cmdlines):
    """Accept only a command line that names the target-g4 build."""
    if not cmdlines:
        raise WrongDaemon("no ceph-rmds is running")
    if any("target-g3/" in line for line in cmdlines):
        raise WrongDaemon("the running daemon is target-g3")
    if not any("target-g4/" in line for line in cmdlines):
        raise WrongDaemon("the running daemon is not target-g4")


def _child(root, relative):
    path = root.joinpath(*relative.split("/"))
    if not path.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"path leaves the scratch directory: {relative}")
    return path


def _call(expected, action):
    try:
        result = action()
    except OSError as error:
        if error.errno in NOT_CEPH and expected == 0:
            raise NotCephfs(os.strerror(error.errno)) from error
        if expected == 0 or error.errno != expected:
            raise
        return None
    if expected != 0:
        raise AssertionError(f"expected errno {expected}, call succeeded")
    return result


def _one(root, step, pinned):
    kind = step[0]
    if kind == "mkdir":
        _call(0, lambda: _child(root, step[1]).mkdir())
    elif kind == "create":
        path, expected = step[1], step[2]

        def create():
            fd = os.open(_child(root, path), os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o644)
            os.close(fd)

        _call(expected, create)
    elif kind == "pin":
        pinned[step[1]] = _child(root, step[1]).stat().st_ino
    elif kind == "same":
        ino = _child(root, step[1]).stat().st_ino
        if pinned.get(step[1]) != ino:
            raise AssertionError(f"inode changed for {step[1]}: {pinned.get(step[1])} -> {ino}")
    elif kind == "setxattr":
        path, name, value, expected = step[1:]
        _call(expected, lambda: os.setxattr(_child(root, path), name, value.encode()))
    elif kind == "getxattr":
        path, name, value = step[1:]
        got = os.getxattr(_child(root, path), name)
        if got.rstrip(b"\x00").decode() != value:
            raise AssertionError(f"{path} {name} is {got!r}, expected {value}")
    elif kind == "removexattr":
        path, name, expected = step[1:]
        _call(expected, lambda: os.removexattr(_child(root, path), name))
    elif kind == "rename":
        src, dst, expected = step[1:]
        _call(expected, lambda: os.rename(_child(root, src), _child(root, dst)))
        if expected == 0:
            pinned[dst] = pinned.pop(src, _child(root, dst).stat().st_ino)
    elif kind == "link":
        dst, src, expected = step[1:]
        _call(expected, lambda: os.link(_child(root, src), _child(root, dst)))
    else:
        raise AssertionError(f"unknown step {kind}")


def exercise(root):
    """Run STEPS in a fresh directory under root. Remove that directory after."""
    root = Path(root)
    if root.is_symlink() or not root.is_dir():
        raise ValueError("scratch root must be a real directory")
    fstype = mount_fstype(root)
    if fstype not in ("ceph", "fuse.ceph-fuse"):
        raise NotCephfs(fstype or "no mount")
    require_g4_daemon(mds_cmdlines())
    scratch = root / f"g4-{os.getpid()}"
    scratch.mkdir()
    pinned = {}
    try:
        for index, step in enumerate(STEPS):
            try:
                _one(scratch, step, pinned)
            except Exception as error:
                raise AssertionError(f"step {index} {step[0]} {step[1]}: {error}") from error
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
