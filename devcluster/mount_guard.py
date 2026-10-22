#!/usr/bin/env python3
"""Read-only mount guards. findmnt JSON preserves decoded paths and mount IDs."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import uuid


class MountIdentityError(ValueError):
    """Required boot-scoped unique mount identity is unavailable or malformed."""


def canonical(path):
    return Path(path).resolve(strict=False)


def mounts():
    result = subprocess.run(
        ["findmnt", "--json", "--list", "-o", "ID,TARGET,SOURCE,FSTYPE,FSROOT,OPTIONS,MAJ:MIN"],
        check=True, text=True, capture_output=True, timeout=10)
    return json.loads(result.stdout)["filesystems"]


def exact_mount(path, table=None):
    return exact_physical_mount(canonical(path), table)


def exact_physical_mount(path, table=None):
    """Look up an already resolved path without statting a failed network mount."""
    path = Path(os.path.normpath(path))
    rows = mounts() if table is None else table
    matches = [row for row in rows if Path(os.path.normpath(row["target"])) == path]
    if len(matches) > 1:
        raise ValueError(f"stacked mounts at {path}; refusing ambiguous ownership")
    return matches[0] if matches else None


def mount_key(row):
    return {key: row[key] for key in ("id", "target", "source", "fstype", "fsroot", "maj:min")}


def boot_identity():
    value = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    parsed = uuid.UUID(value)
    if parsed.int == 0 or str(parsed) != value:
        raise ValueError("invalid host boot UUID")
    return value


def capture_mount_identity(path):
    """Capture a physically resolved mount. No fallback to recycled mountinfo IDs.

    Linux STATMOUNT_MNT_BASIC / listmount exposes the boot-scoped 64-bit unique
    mount ID through findmnt UNIQ-ID. Keep the boot UUID with persisted evidence.
    Mount metadata and a second mountinfo read guard ordinary mount replacement.
    """
    path = Path(os.path.normpath(path))
    row = exact_physical_mount(path)
    if row is None:
        raise MountIdentityError("expected test mount is absent")
    try:
        boot = boot_identity()
        result = subprocess.run(
            ["findmnt", "--kernel=listmount", "--json", "--mountpoint", str(path), "-o", "TARGET,UNIQ-ID"],
            check=True, text=True, capture_output=True, timeout=10)
        rows = json.loads(result.stdout)["filesystems"]
        if not isinstance(rows, list) or len(rows) != 1 or Path(os.path.normpath(rows[0]["target"])) != path:
            raise ValueError("unique mount query did not identify exactly the requested mount")
        unique_id = rows[0]["uniq-id"]
        if type(unique_id) is not int or not 0 < unique_id < 1 << 64:
            raise ValueError("unique mount ID must be a nonzero u64 JSON integer")
        if boot_identity() != boot:
            raise ValueError("host boot identity changed during mount capture")
        after = exact_physical_mount(path)
        if after is None or mount_key(after) != mount_key(row):
            raise ValueError("mount changed during unique identity capture")
        return {**mount_key(row), "boot_id": boot, "unique_id": unique_id}
    except (OSError, subprocess.SubprocessError, ValueError, KeyError, TypeError) as error:
        raise MountIdentityError(f"required kernel unique mount identity unavailable or malformed: {error}") from error


def require_same_mount(path, captured):
    if capture_mount_identity(path) != captured:
        raise MountIdentityError("test mount identity changed; refusing operation or cleanup")


def require_unmounted(path, table=None):
    if exact_mount(path, table) is not None:
        raise ValueError(f"existing mount at {canonical(path)}")
    return canonical(path)


def ceph_mountinfo(text, namespace):
    """Decode the mountinfo ABI without statting mounted filesystem paths."""
    rows = []
    for line in text.splitlines():
        fields = line.split()
        split = fields.index("-")
        if fields[split + 1] in ("ceph", "fuse.ceph-fuse"):
            target = re.sub(r"\\([0-7]{3})", lambda m: chr(int(m[1], 8)), fields[4])
            rows.append({"namespace": namespace, "id": int(fields[0]), "target": target,
                         "fstype": fields[split + 1]})
    return rows


def all_ceph_mounts():
    """Audit all visible mount namespaces. Unreadable namespaces fail closed."""
    rows, seen = [], set()
    for proc in Path("/proc").iterdir():
        if not proc.name.isdecimal():
            continue
        try:
            namespace = os.readlink(proc / "ns/mnt")
            if namespace in seen:
                continue
            text = (proc / "mountinfo").read_text()
        except (FileNotFoundError, ProcessLookupError):
            continue
        except PermissionError as error:
            raise ValueError("cannot audit every mount namespace; run the trial with sufficient privilege") from error
        seen.add(namespace)
        rows.extend(ceph_mountinfo(text, namespace))
    return rows


def external_path(mount, path):
    """Resolve existing parents before opening. Also reject same-device bind aliases."""
    mount = canonical(mount)
    if Path(path).is_symlink():
        raise ValueError("history/artifact path must not itself be a symlink")
    path = canonical(path)
    parent = path.parent.resolve(strict=True)
    if path.is_relative_to(mount) or parent.stat().st_dev == mount.stat().st_dev:
        raise ValueError("history/artifacts must be on a different filesystem outside the test mount")
    # An independent CephFS/NFS mount is not an external controller disk either.
    covering = [r for r in mounts() if parent.is_relative_to(Path(r["target"]))]
    row = max(covering, key=lambda r: len(r["target"]))
    if row["fstype"] in ("ceph", "fuse.ceph-fuse", "nfs", "nfs4"):
        raise ValueError("history/artifacts must be on a local controller filesystem")
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("canonical", "unmounted", "capture", "same"))
    parser.add_argument("path")
    parser.add_argument("--identity", type=Path)
    args = parser.parse_args()
    if args.mode == "canonical":
        print(canonical(args.path))
    elif args.mode == "unmounted":
        print(require_unmounted(args.path))
    elif args.mode == "capture":
        print(json.dumps(capture_mount_identity(canonical(args.path))))
    else:
        if args.identity is None:
            parser.error("--identity is required")
        require_same_mount(canonical(args.path), json.loads(args.identity.read_text()))


if __name__ == "__main__":
    main()
