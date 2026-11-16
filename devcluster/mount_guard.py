#!/usr/bin/env python3
"""Mount guards. findmnt JSON preserves decoded paths and mount IDs."""
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


# Kernels before listmount/statmount have no UNIQ-ID. Assignments remember which
# live kernel client belongs to a mount. The id is the monitor global id, never
# the mountinfo id, which a remount can reuse.
CLIENT_ASSIGNMENTS = Path("/run/rust-mds-mount-unique.json")


class _ListMountUnsupported(Exception):
    """This findmnt build has no ``--kernel=listmount`` query."""


def _u64(value):
    if type(value) is not int or not 0 < value < 1 << 64:
        raise ValueError("unique mount ID must be a nonzero u64 JSON integer")
    return value


def _listmount_unique_id(path):
    try:
        result = subprocess.run(
            ["findmnt", "--kernel=listmount", "--json", "--mountpoint", str(path), "-o", "TARGET,UNIQ-ID"],
            check=True, text=True, capture_output=True, timeout=10)
    except subprocess.CalledProcessError as error:
        err = error.stderr or ""
        if any(phrase in err for phrase in (
                "doesn't allow an argument", "unrecognized option", "unknown option", "invalid option")):
            raise _ListMountUnsupported() from error
        raise
    rows = json.loads(result.stdout)["filesystems"]
    if not isinstance(rows, list) or len(rows) != 1 or Path(os.path.normpath(rows[0]["target"])) != path:
        raise ValueError("unique mount query did not identify exactly the requested mount")
    return _u64(rows[0]["uniq-id"])


def _mount_fsid(row):
    options = dict(part.split("=", 1) for part in row.get("options", "").split(",") if "=" in part)
    fsid = options.get("fsid", "").strip()
    if not fsid or any(character in fsid for character in " /,"):
        raise ValueError("kernel ceph mount has no fsid option")
    return fsid.lower()


def _live_kernel_clients(fsid):
    root = Path("/sys/kernel/debug/ceph")
    try:
        mounted = root.is_dir()
    except PermissionError:
        mounted = False
    if not mounted and os.geteuid() == 0:
        subprocess.run(["mount", "-t", "debugfs", "none", "/sys/kernel/debug"], check=False, timeout=10,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if not root.is_dir():
        raise ValueError("kernel ceph debugfs is unavailable")
    clients = set()
    prefix = fsid + ".client"
    for child in root.iterdir():
        name = child.name
        if not name.lower().startswith(prefix) or not name[len(prefix):].isdecimal():
            continue
        clients.add(int(name[len(prefix):]))
    return clients


def _load_client_assignments(boot):
    path = CLIENT_ASSIGNMENTS
    if path.is_symlink() or not path.is_file():
        return {}
    data = json.loads(path.read_text())
    if data.get("boot_id") != boot or not isinstance(data.get("kernel"), list):
        return {}
    assignments = {}
    for item in data["kernel"]:
        assignments[(item["path"], int(item["mount_id"]), item["dev"])] = _u64(int(item["client"]))
    return assignments


def _save_client_assignments(boot, assignments):
    path = CLIENT_ASSIGNMENTS
    if path.is_symlink():
        raise ValueError("mount identity store must not be a symlink")
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {"boot_id": boot, "kernel": [
        {"path": key[0], "mount_id": key[1], "dev": key[2], "client": client}
        for key, client in sorted(assignments.items())]}
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(json.dumps(payload))
    os.chmod(tmp, 0o600)
    os.replace(tmp, path)


def _kernel_ceph_unique_id(path, row, boot):
    clients = _live_kernel_clients(_mount_fsid(row))
    key = (str(path), int(row["id"]), row["maj:min"])
    assignments = {item: client for item, client in _load_client_assignments(boot).items() if client in clients}
    if key in assignments:
        return assignments[key]
    free = sorted(set(clients) - set(assignments.values()))
    if len(free) != 1:
        raise ValueError("kernel ceph client id is not bound to exactly one new mount")
    assignments[key] = free[0]
    _save_client_assignments(boot, assignments)
    return free[0]


def _fuse_instance_id(path):
    path = Path(os.path.normpath(path))
    found = []
    for proc in Path("/proc").iterdir():
        if not proc.name.isdecimal():
            continue
        try:
            argv = [os.fsdecode(part) for part in (proc / "cmdline").read_bytes().split(b"\0") if part]
            if not argv or Path(argv[0]).name != "ceph-fuse":
                continue
            if canonical(argv[-1]) != path:
                continue
            start = int((proc / "stat").read_text().rsplit(")", 1)[1].split()[19])
            found.append((int(proc.name) << 32) | (start & 0xFFFFFFFF))
        except (OSError, IndexError, ValueError):
            continue
    if len(found) != 1:
        raise ValueError("ceph-fuse mount has no single process identity")
    return _u64(found[0])


def _unique_id_without_listmount(path, row, boot):
    if row["fstype"] == "ceph":
        return _kernel_ceph_unique_id(path, row, boot)
    if row["fstype"] == "fuse.ceph-fuse":
        return _fuse_instance_id(path)
    raise ValueError("this kernel has no unique mount id for " + row["fstype"])


def capture_mount_identity(path):
    """Capture a physically resolved mount. No fallback to recycled mountinfo IDs.

    Linux STATMOUNT_MNT_BASIC / listmount exposes the boot-scoped 64-bit unique
    mount ID through findmnt UNIQ-ID. Keep the boot UUID with persisted evidence.
    Mount metadata and a second mountinfo read guard ordinary mount replacement.
    A findmnt without listmount uses the kernel client's monitor global id, or
    the ceph-fuse process instance. Those values change for a new mount.
    """
    path = Path(os.path.normpath(path))
    row = exact_physical_mount(path)
    if row is None:
        raise MountIdentityError("expected test mount is absent")
    try:
        boot = boot_identity()
        try:
            unique_id = _listmount_unique_id(path)
        except _ListMountUnsupported:
            unique_id = _unique_id_without_listmount(path, row, boot)
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


def _pid_in_mount_namespace(namespace):
    for proc in Path("/proc").iterdir():
        if not proc.name.isdecimal():
            continue
        try:
            if os.readlink(proc / "ns/mnt") == namespace:
                return int(proc.name)
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue
    return None


def isolate_mount(path):
    """Keep this mount out of every other mount namespace.

    A shared parent copies a new CephFS mount into peer namespaces. Each copy
    keeps a client session, so a later remount is not a cold client. Make this
    mount private, then unmount only the copies. The caller's mount stays.
    """
    path = str(canonical(path))
    if exact_mount(path) is None:
        raise MountIdentityError("expected test mount is absent")
    subprocess.run(["mount", "--make-private", path], check=True, timeout=10)
    subprocess.run(["mount", "--make-unbindable", path], check=True, timeout=10)
    mine = os.readlink("/proc/self/ns/mnt")
    for row in all_ceph_mounts():
        if row["namespace"] == mine or os.path.normpath(row["target"]) != os.path.normpath(path):
            continue
        pid = _pid_in_mount_namespace(row["namespace"])
        if pid is None:
            continue
        subprocess.run(["nsenter", "-t", str(pid), "-m", "--", "umount", path], check=True, timeout=10)
    leftover = [row for row in all_ceph_mounts()
                if os.path.normpath(row["target"]) == os.path.normpath(path) and row["namespace"] != mine]
    if leftover:
        raise MountIdentityError(f"mount remains in another namespace: {leftover}")
    if exact_mount(path) is None:
        raise MountIdentityError("isolating other namespaces unmounted the caller")


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
    parser.add_argument("mode", choices=("canonical", "unmounted", "capture", "same", "isolate"))
    parser.add_argument("path")
    parser.add_argument("--identity", type=Path)
    args = parser.parse_args()
    if args.mode == "canonical":
        print(canonical(args.path))
    elif args.mode == "unmounted":
        print(require_unmounted(args.path))
    elif args.mode == "capture":
        print(json.dumps(capture_mount_identity(canonical(args.path))))
    elif args.mode == "isolate":
        isolate_mount(args.path)
        print(json.dumps(capture_mount_identity(canonical(args.path))))
    else:
        if args.identity is None:
            parser.error("--identity is required")
        require_same_mount(canonical(args.path), json.loads(args.identity.read_text()))


if __name__ == "__main__":
    main()
