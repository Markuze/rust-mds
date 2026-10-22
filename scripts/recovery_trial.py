#!/usr/bin/env python3
"""G2 clean restart and partial G3 recovery. Preflight has no mutating effects."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import select
import signal
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "devcluster"))
from mount_guard import (all_ceph_mounts, canonical, capture_mount_identity,
                         exact_physical_mount as exact_mount, external_path, mount_key,
                         require_same_mount, require_unmounted)


class NotActive(ValueError):
    pass


class Prerequisite(ValueError):
    pass


class Cancelled(Exception):
    pass


def write_json(path, value):
    with path.open("x") as file:
        json.dump(value, file, indent=2)
        file.flush()
        os.fsync(file.fileno())
    fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def failure_code(error, trial):
    if not trial.workload_started and (isinstance(error, (Prerequisite, BlockingIOError)) or
            (not trial.preflight_complete and isinstance(error, (ValueError, FileNotFoundError, PermissionError)))):
        return 2
    return 1


def command(args, timeout=20):
    # Only this newly created command session is subject to timeout cancellation.
    # No PID/name search or unrelated daemon group is ever signaled.
    child = subprocess.Popen(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                             start_new_session=True)
    try:
        try:
            stdout, stderr = child.communicate(timeout=timeout)
        except BaseException as error:
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                child.communicate(timeout=2)
            except subprocess.TimeoutExpired:
                # A task stuck in kernel I/O cannot be reaped yet. Do not block
                # the controller or signal any other process to make it exit.
                raise TimeoutError(f"owned command PID {child.pid} remains blocked after timeout")
            if isinstance(error, subprocess.TimeoutExpired):
                raise TimeoutError(f"owned command PID {child.pid} exceeded {timeout}s") from error
            raise
        if child.returncode:
            raise subprocess.CalledProcessError(child.returncode, args, stdout, stderr)
        return stdout
    finally:
        child.stdout.close()
        child.stderr.close()


def cluster_state(conf, allow_inactive=False, timeout=20):
    fs = json.loads(command(["ceph", "-c", str(conf), "fs", "get", "a", "-f", "json"], timeout=timeout))
    m = fs["mdsmap"]
    identity = {"fsid": command(["ceph", "-c", str(conf), "fsid"], timeout=timeout).strip(),
                "fscid": fs["id"], "name": m["fs_name"],
                "metadata_pool": m["metadata_pool"], "data_pools": m["data_pools"]}
    gid = m["up"].get("mds_0")
    active = m["info"].get(f"gid_{gid}")
    if not active or active["rank"] != 0 or active["state"] != "up:active":
        if allow_inactive:
            return identity, None
        raise NotActive("filesystem a has no active rank-0 MDS")
    return identity, active


def validate_mount(row, identity, fscid):
    # FUSE identity is not exposed by mountinfo. Refuse it until a client-side
    # identity probe exists. Do not infer filesystem identity from a path label.
    if row is None or row["fstype"] != "ceph" or row["fsroot"] != "/":
        raise ValueError("trial requires an identifiable root kernel CephFS mount")
    options = dict(part.split("=", 1) for part in row["options"].split(",") if "=" in part)
    if options.get("fsid") != identity["fsid"] or options.get("mds_namespace") != identity["name"]:
        raise ValueError("mount belongs to another cluster or filesystem")
    if fscid != identity["fscid"]:
        raise ValueError("mount filesystem ID does not match the monitor")


def mounted_fscid(mount):
    # Linux CephFS exposes fscid in the upper 32 bits of statvfs.f_fsid.
    # Probe in a bounded child: an old disconnected mount can block statvfs.
    return int(command([sys.executable, "-c", "import os,sys; print(os.statvfs(sys.argv[1]).f_fsid >> 32)",
                        str(mount)], timeout=10))


def process_identity(pid):
    proc = Path(f"/proc/{pid}")
    argv = [os.fsdecode(v) for v in (proc / "cmdline").read_bytes().split(b"\0") if v]
    exe_path = (proc / "exe").resolve(strict=True)
    exe = exe_path.name
    start = (proc / "stat").read_text().rsplit(")", 1)[1].split()[19]
    inodes = set()
    for fd in (proc / "fd").iterdir():
        try:
            target = os.readlink(fd)
        except FileNotFoundError:
            continue
        if target.startswith("socket:["):
            inodes.add(target[8:-1])
    listeners = set()
    for line in (proc / "net/tcp").read_text().splitlines()[1:]:
        fields = line.split()
        if fields[3] == "0A" and fields[9] in inodes:
            address, port = fields[1].split(":")
            ip = ".".join(str(v) for v in bytes.fromhex(address)[::-1])
            listeners.add(f"{ip}:{int(port, 16)}")
    return {"exe": exe, "exe_path": str(exe_path), "argv": argv, "start": start, "listeners": listeners}


def validate_target(process, active, conf):
    if process["exe"] not in ("ceph-rmds", "ceph-mds"):
        raise ValueError("target is not an MDS")
    argv = process["argv"]
    configs = [argv[i + 1] for i, arg in enumerate(argv[:-1]) if arg in ("-c", "--conf")]
    ids = [argv[i + 1] for i, arg in enumerate(argv[:-1]) if arg in ("-i", "--id")]
    if not configs or canonical(configs[-1]) != canonical(conf) or ids != [active["name"]]:
        raise ValueError("target config/name does not match active rank 0")
    endpoints = {a["addr"] for a in active["addrs"]["addrvec"]}
    if not endpoints.intersection(process["listeners"]):
        raise ValueError("target does not own an active rank-0 listening socket (standby refused)")


class Trial:
    def __init__(self, gate, pid, mount, out, conf, restart, owned_mount, *,
                 binary=None, keyring=None, rust_filesystem=False, takeover_pid=None,
                 cold_mount=None, fuse_mount=None, stop="term", active_timeout=120, expected_fscid=None,
                 inflight=False):
        self.gate, self.pid = gate, pid
        self.mount, self.out, self.conf = canonical(mount), canonical(out), canonical(conf)
        self.restart, self.owned_mount = restart, owned_mount
        self.snapshot = None
        self.pidfd = None
        self.workload_started = False
        self.preflight_complete = False
        self.binary = canonical(binary) if binary else None
        self.keyring = canonical(keyring) if keyring else None
        self.binary_hash = None
        self.rust_filesystem = rust_filesystem
        self.takeover_pid = takeover_pid
        self.cold_mount = canonical(cold_mount) if cold_mount else None
        self.fuse_mount = canonical(fuse_mount) if fuse_mount else None
        self.stop = stop
        self.active_timeout = active_timeout
        self.children = []
        self.extra_mounts = {}
        self.namespace = os.readlink("/proc/self/ns/mnt")
        self.original_mount_id = None
        self.original_mount_identity = None
        self.fuse_process = None
        self.expected_fscid = expected_fscid
        self.inflight = inflight
        self.workload_child = None
        self.inflight_snapshot = None
        self.standby = None
        self.artifacts_created = False
        self.conf_hash = None
        self.phase = "preflight"

    def audit_mounts(self, allowed=()):
        allowed = set(allowed)
        unexpected = [r for r in all_ceph_mounts() if (r["namespace"], r["id"]) not in allowed]
        if unexpected:
            raise Prerequisite(f"other CephFS mount namespaces prevent a cold-client proof: {unexpected}")

    def check_mount(self):
        row = exact_mount(self.mount)
        if self.snapshot is not None and (row is None or mount_key(row) != mount_key(self.snapshot)):
            raise ValueError("mount identity changed; refusing operation or cleanup")
        if self.snapshot is not None:
            require_same_mount(self.mount, self.snapshot)
        validate_mount(row, self.identity, mounted_fscid(self.mount))
        return row

    def preflight(self):
        if not self.owned_mount:
            raise ValueError("--owned-mount is required; caller must own this test mount")
        self.identity, self.active = cluster_state(self.conf)
        if self.expected_fscid is not None and self.identity["fscid"] != self.expected_fscid:
            raise Prerequisite("active filesystem is not the caller-designated filesystem ID")
        row = exact_mount(self.mount)
        validate_mount(row, self.identity, mounted_fscid(self.mount))
        before = process_identity(self.pid)
        validate_target(before, self.active, self.conf)
        if before["exe"] != "ceph-rmds" or not self.rust_filesystem:
            raise Prerequisite("a caller-designated Rust filesystem and --rust-filesystem are required")
        # Required before workload/admission and before arming cleanup. Legacy
        # mountinfo IDs can be recycled immediately after a normal unmount.
        self.original_mount_identity = capture_mount_identity(self.mount)
        if mount_key(self.original_mount_identity) != mount_key(row):
            raise Prerequisite("mount changed during preflight")
        if self.restart:
            raise Prerequisite("external restart commands are not permitted; select --binary or --takeover-pid")
        if self.gate == "g2" and (self.takeover_pid or self.cold_mount or self.fuse_mount or self.inflight):
            raise Prerequisite("G2 uses one cold kernel remount and explicit daemon restart; extra clients are G3 options")
        self.binary = self.binary or canonical(before["exe_path"])
        if self.binary.name != "ceph-rmds" or not self.binary.is_file() or not os.access(self.binary, os.X_OK):
            raise Prerequisite("--binary must select an executable ceph-rmds")
        self.binary_hash = hashlib.sha256(self.binary.read_bytes()).hexdigest()
        self.conf_hash = hashlib.sha256(self.conf.read_bytes()).hexdigest()
        if not self.takeover_pid and (self.keyring is None or not self.keyring.is_file()):
            raise Prerequisite("--keyring must select the restart daemon's existing keyring")
        if self.fuse_mount is not None:
            if shutil.which("ceph-fuse") is None or not Path(os.environ.get("CEPH_KEYRING", "/missing-admin-keyring")).is_file():
                raise Prerequisite("owned FUSE creation requires the existing admin keyring")
        self.original_mount_id = row["id"]
        self.audit_mounts([(self.namespace, row["id"])])
        if self.gate == "g3" and self.cold_mount is None:
            raise Prerequisite("G3 requires --cold-mount for independent post-recovery verification")
        for candidate in (self.cold_mount, self.fuse_mount):
            if candidate is not None:
                require_unmounted(candidate)
                if not candidate.is_dir() or any(candidate.iterdir()) or candidate.is_relative_to(self.mount):
                    raise Prerequisite("extra client targets must be existing empty local directories outside the test mount")
                external_path(self.mount, candidate / "guard")
        if len({p for p in (self.mount, self.cold_mount, self.fuse_mount) if p}) != len([p for p in (self.mount, self.cold_mount, self.fuse_mount) if p]):
            raise Prerequisite("client mount targets must be distinct physical paths")
        self.standby = self.check_standby() if self.takeover_pid else None
        if not self.takeover_pid:
            fsmap = json.loads(command(["ceph", "-c", str(self.conf), "fs", "dump", "-f", "json"]))
            if fsmap.get("standbys"):
                raise Prerequisite("unselected standby can race explicit restart; select --takeover-pid or isolate the trial")
        external_path(self.mount, self.out)
        if self.out.exists():
            raise Prerequisite("artifact directory already exists; choose a new exclusive trial directory")
        self.pidfd = os.pidfd_open(self.pid)
        after = process_identity(self.pid)
        validate_target(after, self.active, self.conf)
        if before["start"] != after["start"]:
            raise ValueError("target process changed during preflight")
        self.process_start = after["start"]
        # Arm ownership only after every read-only check succeeds.
        self.snapshot = self.original_mount_identity
        self.preflight_complete = True

    def check_standby(self):
        if self.takeover_pid == self.pid:
            raise Prerequisite("takeover PID must differ from active PID")
        process = process_identity(self.takeover_pid)
        data = json.loads(command(["ceph", "-c", str(self.conf), "fs", "dump", "-f", "json"]))
        matches = []
        for standby in data.get("standbys", []):
            if standby.get("state") not in ("up:standby", "up:standby-replay"):
                continue
            try:
                validate_target(process, standby, self.conf)
            except ValueError:
                continue
            matches.append(standby)
        if process["exe"] != "ceph-rmds" or len(matches) != 1:
            raise Prerequisite("takeover PID is not one identified cold Rust standby")
        if matches[0].get("join_fscid", -1) not in (-1, self.identity["fscid"]):
            raise Prerequisite("selected standby is assigned to another filesystem")
        if len(data.get("standbys", [])) != 1 or hashlib.sha256(Path(f"/proc/{self.takeover_pid}/exe").read_bytes()).hexdigest() != self.binary_hash:
            raise Prerequisite("takeover requires one selected standby running the designated binary")
        return {"start": process["start"], "gid": matches[0]["gid"], "binary_sha256": self.binary_hash}

    def check_target(self):
        if self.conf_hash is not None and hashlib.sha256(self.conf.read_bytes()).hexdigest() != self.conf_hash:
            raise ValueError("daemon configuration changed during trial")
        identity, active = cluster_state(self.conf)
        if identity != self.identity or active["gid"] != self.active["gid"]:
            raise ValueError("filesystem identity or active MDS changed")
        process = process_identity(self.pid)
        validate_target(process, active, self.conf)
        if process["start"] != self.process_start:
            raise ValueError("target process instance changed")

    def unmount_clean(self):
        self.check_mount()
        # G2 must prove clean client shutdown. No force or lazy detach is allowed.
        # If this fails, do not retry through failure cleanup.
        self.snapshot = None
        command(["sudo", "umount", "--", str(self.mount)], timeout=60)
        require_unmounted(self.mount)
        self.audit_mounts()

    def cleanup(self):
        if self.workload_child is not None:
            self.stop_owned_child(self.workload_child)
        for path, identity in list(self.extra_mounts.items()):
            try:
                require_same_mount(path, identity)
                command(["sudo", "umount", "--", str(path)], timeout=30)
                del self.extra_mounts[path]
            except Exception as error:
                print(f"extra client mount retained at {path}: {error}", file=sys.stderr)
        if self.snapshot is not None and self.workload_started:
            try:
                self.check_mount()
                command(["sudo", "umount", "--", str(self.mount)], timeout=30)
                self.snapshot = None
            except Exception as error:
                print(f"cleanup left mount unchanged or mounted: {error}", file=sys.stderr)
        candidates = [p for p in (self.mount, self.cold_mount, self.fuse_mount) if p is not None]
        unknown_mount = any(exact_mount(p) is not None for p in candidates) if self.children else False
        if unknown_mount:
            print("owned daemon/client children retained while a test path remains mounted", file=sys.stderr)
        if self.snapshot is None and not self.extra_mounts and not unknown_mount:
            for child in self.children:
                self.stop_owned_child(child)

    def stop_owned_child(self, child):
        if child.poll() is not None:
            return
        try:
            fd = os.pidfd_open(child.pid)
        except ProcessLookupError:
            child.poll()
            return
        try:
            try:
                signal.pidfd_send_signal(fd, signal.SIGTERM)
            except ProcessLookupError:
                child.poll()
                return
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                signal.pidfd_send_signal(fd, signal.SIGKILL)
                try:
                    child.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    print(f"owned child PID {child.pid} remains blocked", file=sys.stderr)
        finally:
            os.close(fd)

    def start_inflight(self):
        history = self.out / "inflight-history.jsonl"
        with (self.out / "inflight-workload.log").open("x") as log:
            self.workload_child = subprocess.Popen([sys.executable, str(ROOT / "scripts/crash-history.py"),
                                                    "record", "--mount", str(self.mount), "--history", str(history)],
                                                   stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if history.exists():
                # Only complete JSON lines are observed. A last intent is not
                # proof that its wire request reached the MDS or committed.
                lines = history.read_text().splitlines(keepends=True)
                events = [json.loads(line) for line in lines if line.endswith("\n")]
                if events and events[-1].get("event") == "intent":
                    self.inflight_snapshot = events
                    write_json(self.out / "inflight-at-stop.json", events)
                    return
            if self.workload_child.poll() is not None:
                raise ValueError("sequential workload finished before a pending intent was observed; no crash-window claim")
            time.sleep(.001)
        raise TimeoutError("sequential workload produced no pending intent before deadline")

    def verify_inflight(self, mount):
        if self.workload_child is None:
            return
        try:
            result = self.workload_child.wait(timeout=120)
        except subprocess.TimeoutExpired:
            self.stop_owned_child(self.workload_child)
            raise TimeoutError("owned sequential workload did not finish after recovery")
        if result:
            raise ValueError("sequential client operation failed across recovery; retained history is not a pass")
        output = command([sys.executable, str(ROOT / "scripts/crash-history.py"), "verify",
                          "--mount", str(mount), "--history", str(self.out / "inflight-history.jsonl")], timeout=120)
        with (self.out / "inflight-verification.log").open("a") as log:
            log.write(output)

    def history(self, mode, mount=None, future=False):
        args = ["sudo", sys.executable, str(ROOT / "scripts/restart_history.py"),
                mode, "--mount", str(mount or self.mount), "--history", str(self.out / "history.jsonl")]
        if future or mode == "allocate":
            args += ["--future-history", str(self.out / "future-history.jsonl")]
        try:
            output = command(args, timeout=120)
        except Exception as error:
            with (self.out / "oracle-output.jsonl").open("a") as log:
                log.write(json.dumps({"mode": mode, "mount": str(mount or self.mount), "error": str(error),
                                      "stdout": getattr(error, "output", None), "stderr": getattr(error, "stderr", None)}) + "\n")
            raise
        with (self.out / "oracle-output.jsonl").open("a") as log:
            log.write(json.dumps({"mode": mode, "mount": str(mount or self.mount), "output": output}) + "\n")
        return json.loads(output)

    def mount_kernel(self, path):
        require_unmounted(path)
        command([str(ROOT / "devcluster/mount-kernel.sh"), str(path), "v1", "noshare"], timeout=90)
        row = exact_mount(path)
        validate_mount(row, self.identity, mounted_fscid(path))
        identity = capture_mount_identity(path)
        if mount_key(identity) != mount_key(row) or identity["boot_id"] != self.original_mount_identity["boot_id"]:
            raise ValueError("mount or host boot identity changed")
        return identity

    def start_fuse(self):
        require_unmounted(self.fuse_mount)
        with (self.out / "fuse.log").open("x") as log:
            child = subprocess.Popen(["ceph-fuse", "-f", "-c", str(self.conf), "-n", "client.admin",
                                      "--keyring", os.environ["CEPH_KEYRING"], "--client_fs=a", str(self.fuse_mount)], stdout=log,
                                     stderr=subprocess.STDOUT, start_new_session=True)
        self.children.append(child)
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            if child.poll() is not None:
                raise ValueError("owned foreground FUSE client exited; see fuse.log")
            row = exact_mount(self.fuse_mount)
            if row:
                process = process_identity(child.pid)
                if process["exe"] != "ceph-fuse" or row["fstype"] != "fuse.ceph-fuse" or row["fsroot"] != "/":
                    raise ValueError("owned FUSE process/mount identity not established")
                if not any(os.readlink(fd) == "/dev/fuse" for fd in Path(f"/proc/{child.pid}/fd").iterdir()):
                    raise ValueError("owned FUSE process has no /dev/fuse connection")
                identity, _ = cluster_state(self.conf)
                if identity != self.identity:
                    raise ValueError("filesystem changed while creating FUSE client")
                captured = capture_mount_identity(self.fuse_mount)
                if mount_key(captured) != mount_key(row) or captured["boot_id"] != self.original_mount_identity["boot_id"]:
                    raise ValueError("FUSE mount or host boot identity changed")
                self.extra_mounts[self.fuse_mount] = captured
                self.fuse_process = {"pid": child.pid, "start": process["start"]}
                return
            time.sleep(.1)
        raise TimeoutError("owned FUSE client did not mount")

    def check_fuse(self):
        require_same_mount(self.fuse_mount, self.extra_mounts[self.fuse_mount])
        process = process_identity(self.fuse_process["pid"])
        if process["exe"] != "ceph-fuse" or process["start"] != self.fuse_process["start"]:
            raise ValueError("owned FUSE process changed")

    def start_daemon(self):
        if hashlib.sha256(self.conf.read_bytes()).hexdigest() != self.conf_hash:
            raise ValueError("restart configuration changed during trial")
        if self.takeover_pid:
            process = process_identity(self.takeover_pid)
            if process["start"] != self.standby["start"]:
                raise ValueError("standby PID instance changed")
            return self.takeover_pid
        if hashlib.sha256(self.binary.read_bytes()).hexdigest() != self.binary_hash:
            raise ValueError("restart executable changed during trial")
        with (self.out / "restart.log").open("x") as log:
            child = subprocess.Popen([str(self.binary), "-i", self.active["name"], "-c", str(self.conf),
                                      "--keyring", str(self.keyring)], stdout=log,
                                     stderr=subprocess.STDOUT, start_new_session=True)
        self.children.append(child)
        return child.pid

    def wait_active(self, pid):
        deadline = time.monotonic() + self.active_timeout
        while time.monotonic() < deadline:
            for child in self.children:
                if child.pid == pid and child.poll() is not None:
                    raise ValueError("owned restart daemon exited; see restart.log")
            budget = min(5, max(.01, (deadline - time.monotonic()) / 2))
            identity, active = cluster_state(self.conf, allow_inactive=True, timeout=budget)
            if identity != self.identity:
                raise ValueError("filesystem/pool identity changed across restart")
            if active is None or active["gid"] == self.active["gid"]:
                # A killed daemon remains in the monitor map until failure
                # detection. Never confuse its stale active entry with recovery.
                time.sleep(.25)
                continue
            process = process_identity(pid)
            validate_target(process, active, self.conf)
            if process["exe"] != "ceph-rmds":
                raise ValueError("recovered active is not the selected Rust MDS")
            if not self.takeover_pid and hashlib.sha256(Path(f"/proc/{pid}/exe").read_bytes()).hexdigest() != self.binary_hash:
                raise ValueError("running restart executable differs from the preflight binary")
            if self.takeover_pid and (process["start"] != self.standby["start"] or active["gid"] != self.standby["gid"]):
                raise ValueError("a different standby took over")
            return time.monotonic_ns(), identity, active
        raise TimeoutError("daemon did not report active before the recovery deadline")

    def run(self):
        self.preflight()
        # Local coordination state is created only after read-only preflight.
        Path("/tmp/opencode").mkdir(exist_ok=True)
        with open("/tmp/opencode/rust-mds-live.lock", "a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            try:
                self.check_mount()
                self.check_target()
                return self.execute()
            except Exception:
                self.cleanup()
                raise

    def execute(self):
        self.out.mkdir()  # Exclusive trial directory. No writes before preflight.
        self.artifacts_created = True
        fd = os.open(self.out.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
        write_json(self.out / "identity-before.json", self.identity)
        write_json(self.out / "client-before.json", {"primary": self.snapshot, "namespace": self.namespace})
        write_json(self.out / "daemon.json", {"binary": str(self.binary), "sha256": self.binary_hash,
                                              "config": str(self.conf), "config_sha256": self.conf_hash,
                                              "original_pid": self.pid, "gate": self.gate, "standby": self.standby})
        self.check_mount()
        self.workload_started = True
        self.phase = "recording"
        self.history("record")
        if self.fuse_mount is not None:
            self.start_fuse()
            self.check_fuse()
            self.history("verify", self.fuse_mount)
        self.check_mount()
        self.check_target()
        if self.fuse_mount is not None:
            self.check_fuse()
        if self.inflight:
            self.start_inflight()
            self.check_mount()
            self.check_target()
        if self.gate == "g2":
            self.unmount_clean()
            self.phase = "client_closed"
            self.check_target()
        stop_signal = signal.SIGTERM if self.gate == "g2" and self.stop == "term" else signal.SIGKILL
        write_json(self.out / "stop-intent.json", {"pid": self.pid, "gid": self.active["gid"], "signal": int(stop_signal)})
        self.phase = "stopping_mds"
        start = time.monotonic_ns()
        signal.pidfd_send_signal(self.pidfd, stop_signal)
        write_json(self.out / "stop-sent.json", {"monotonic_ns": start, "pid": self.pid})
        poller = select.poll()
        poller.register(self.pidfd, select.POLLIN)
        if not poller.poll(60000):
            raise TimeoutError("MDS did not stop; no escalation performed")
        recovered_pid = self.start_daemon()
        self.phase = "waiting_active"
        active_at, recovered_identity, recovered_active = self.wait_active(recovered_pid)
        write_json(self.out / "identity-after.json", recovered_identity)
        write_json(self.out / "recovered-active.json", {"pid": recovered_pid, "gid": recovered_active["gid"]})
        if self.gate == "g2":
            self.phase = "cold_remount"
            self.audit_mounts()
            self.snapshot = self.mount_kernel(self.mount)
            if self.snapshot["boot_id"] != self.original_mount_identity["boot_id"]:
                raise ValueError("host boot identity changed across cold remount")
            if self.snapshot["unique_id"] == self.original_mount_identity["unique_id"]:
                raise ValueError("cold remount retained the original unique mount identity")
        else:
            self.check_mount()
        self.phase = "verifying"
        self.history("verify")
        self.phase = "future_allocation"
        allocated = self.history("allocate")
        self.history("verify", future=True)
        self.verify_inflight(self.mount)
        if self.fuse_mount is not None:
            self.check_fuse()
            self.history("verify", self.fuse_mount, future=True)
        if self.gate == "g3":
            self.extra_mounts[self.cold_mount] = self.mount_kernel(self.cold_mount)
            self.history("verify", self.cold_mount, future=True)
            self.verify_inflight(self.cold_mount)
        timings = {"active_seconds": (active_at - start) / 1e9,
                   "first_success_seconds": (allocated["first_success_monotonic_ns"] - start) / 1e9,
                   "total_seconds": (time.monotonic_ns() - start) / 1e9,
                   "first_success_operation": "post-recovery O_EXCL create/write/file-fsync/parent-fsync"}
        write_json(self.out / "timings.json", timings)
        result = {"gate": self.gate, "result": "PASS" if self.gate == "g2" else "PARTIAL", "identity": self.identity,
                  "coverage": "clean-restart-application" if self.gate == "g2" else "mounted-recovery-foundation",
                  "recovered_pid": recovered_pid, "future_allocations": allocated["future_allocations"], "timings": timings,
                  "pending_intent_observed": self.inflight_snapshot is not None,
                  "same_wire_tid_retry_proven": False}
        write_json(self.out / "client-state.json", {"primary": self.snapshot,
                                                    "extra_mounts": {str(p): row for p, row in self.extra_mounts.items()},
                                                    "fuse_process": self.fuse_process})
        write_json(self.out / "result.json", result)
        self.phase = "complete"
        print(json.dumps(result))
        return 0 if self.gate == "g2" else 77


def main():
    if os.environ.get("RMDS_RECOVERY_READY") != "yes":
        print("DEPENDENCY: persistence/reconnect integration not enabled; no MDS was stopped")
        return 77
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("gate", choices=("g2", "g3"))
    parser.add_argument("pid", type=int)
    parser.add_argument("mount", type=Path)
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("--owned-mount", action="store_true")
    argv = sys.argv[1:]
    parser.add_argument("--rust-filesystem", action="store_true")
    parser.add_argument("--expected-fscid", required=True, type=int)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--keyring", type=Path, default=os.environ.get("RMDS_KEYRING"))
    parser.add_argument("--takeover-pid", type=int)
    parser.add_argument("--cold-mount", type=Path)
    parser.add_argument("--fuse-mount", type=Path)
    parser.add_argument("--stop", choices=("term", "kill"), default="term")
    parser.add_argument("--active-timeout", type=int, default=120)
    parser.add_argument("--inflight", action="store_true")
    if "--" in argv:
        parser.error("external restart commands are no longer accepted; use --binary or --takeover-pid")
    args = parser.parse_args(argv)
    if args.gate == "g3" and os.environ.get("RMDS_G3_READY") != "yes":
        print("DEPENDENCY: separate reviewed G3 integration readiness is required; no MDS was stopped")
        return 77
    if os.geteuid() != 0:
        print("PREREQUISITE: run the controller as root to audit every mount namespace and own child cancellation", file=sys.stderr)
        return 2
    capabilities = next(line.split()[1] for line in Path("/proc/self/status").read_text().splitlines()
                        if line.startswith("CapEff:"))
    required = (1 << 21) | (1 << 5)
    if int(capabilities, 16) & required != required:
        print("PREREQUISITE: controller needs CAP_SYS_ADMIN and CAP_KILL", file=sys.stderr)
        return 2
    if not 1 <= args.active_timeout <= 600:
        parser.error("--active-timeout must be 1..600 seconds")
    trial = Trial(args.gate, args.pid, args.mount, args.artifacts,
                  os.environ["CEPH_CONF"], [], args.owned_mount, binary=args.binary,
                  keyring=args.keyring, rust_filesystem=args.rust_filesystem, takeover_pid=args.takeover_pid,
                  cold_mount=args.cold_mount, fuse_mount=args.fuse_mount, stop=args.stop,
                  active_timeout=args.active_timeout, expected_fscid=args.expected_fscid, inflight=args.inflight)
    def cancelled(signum, frame):
        raise Cancelled(f"controller received signal {signum}; cancel only owned work")
    signal.signal(signal.SIGTERM, cancelled)
    signal.signal(signal.SIGINT, cancelled)
    try:
        return trial.run()
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr)
        code = failure_code(error, trial)
        if trial.artifacts_created:
            try:
                write_json(trial.out / "error.json", {"exit": code, "error": str(error),
                                                      "phase": trial.phase, "mount_path": str(trial.mount),
                                                      "original_mount_id": trial.original_mount_id,
                                                      "original_mount_identity": trial.original_mount_identity,
                                                      "primary_mount": trial.snapshot,
                                                      "owned_children": [child.pid for child in trial.children],
                                                      "owned_workload_pid": trial.workload_child.pid if trial.workload_child else None})
            except OSError as artifact_error:
                print(f"cannot retain error.json: {artifact_error}", file=sys.stderr)
        return code
    finally:
        if trial.pidfd is not None:
            os.close(trial.pidfd)


if __name__ == "__main__":
    sys.exit(main())
