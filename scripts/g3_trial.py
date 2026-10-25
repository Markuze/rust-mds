#!/usr/bin/env python3
"""Scoped mounted-application recovery gates. Never a blanket full-G3 PASS."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import select
import shutil
import signal
import stat
import subprocess
import sys
import time

from recovery_trial import (Cancelled, Prerequisite, ROOT, Trial, cluster_state,
                            command, failure_code, process_identity, validate_target,
                            write_json)
from mount_guard import (all_ceph_mounts, canonical, capture_mount_identity,
                         exact_physical_mount, external_path, require_same_mount,
                         require_unmounted)
from g3_client import selected_cases, excluded_cases


def persistence_settings(text, daemon_name):
    """Match CephConf::parse/get and u64 settings in crates/mds/src/config.rs.

    Keys replace each ASCII space/dash with underscore. Sections and keys use
    ASCII lowercase. Numeric values have no suffix units; interval is seconds.
    """
    lower = str.maketrans("ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz")
    if any(character in text for character in "\x1c\x1d\x1e\x1f"):
        raise Prerequisite("unsupported configuration control separators")
    sections, section = {}, "global"
    # Rust str::lines splits LF/CRLF, not Python's extra Unicode separators.
    for number, raw in enumerate(text.split("\n"), 1):
        line = re.split(r"[#;]", raw, maxsplit=1)[0].strip()
        if not line:
            continue
        if line.startswith("["):
            if not line.endswith("]"):
                raise Prerequisite(f"invalid daemon config section at line {number}")
            section = line[1:-1].strip().translate(lower)
        elif "=" in line:
            key, value = line.split("=", 1)
            key = key.strip().translate(lower).replace(" ", "_").replace("-", "_")
            sections.setdefault(section, {})[key] = value.strip().strip('"')
        else:
            raise Prerequisite(f"unsupported/invalid daemon config at line {number}; includes are not expanded")
    settings = {}
    for key, default in (("rmds_max_checkpoint_generations", 64), ("rmds_checkpoint_interval", 30),
                         ("rmds_checkpoint_records", 16)):
        value = next((sections[name][key] for name in (f"mds.{daemon_name}", "mds", "global")
                      if key in sections.get(name, {})), str(default))
        digits = value.lstrip("+").lstrip("0") or "0"
        if (not re.fullmatch(r"\+?[0-9]+", value) or len(digits) > 20 or
                (len(digits) == 20 and digits > "18446744073709551615")):
            raise Prerequisite(f"invalid daemon u64 setting {key}={value!r}; suffix units are unsupported")
        settings[key] = int(digits)
    if (settings["rmds_max_checkpoint_generations"] < 2 or settings["rmds_checkpoint_interval"] == 0 or
            settings["rmds_checkpoint_records"] == 0):
        raise Prerequisite("invalid bounded daemon checkpoint settings")
    return settings


def require_assertions(result, cases):
    assertions = result.get("assertions", {})
    if any(assertions.get(case) is not True for case in cases):
        raise ValueError(f"selected application assertion missing/false: {assertions}")


def events(path):
    if not path.exists():
        return []
    # A concurrently written last line is not an observed event yet.
    return [json.loads(line) for line in path.read_text().splitlines(keepends=True) if line.endswith("\n")]


class ClientProcess:
    """One bounded command at a time. All filesystem I/O runs in the child."""

    def __init__(self, child):
        self.child, self.seq, self.pending, self.buffer = child, 0, False, b""

    def send(self, action, **args):
        if self.pending or self.child.poll() is not None:
            raise ValueError("client has a pending command or exited")
        data = json.dumps({"seq": self.seq, "action": action, "args": args}).encode() + b"\n"
        self.child.stdin.write(data)
        self.child.stdin.flush()
        self.pending = True

    def receive(self, timeout):
        deadline = time.monotonic() + timeout
        while b"\n" not in self.buffer:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"owned client PID {self.child.pid} command deadline exceeded")
            if not select.select([self.child.stdout], [], [], min(remaining, .1))[0]:
                if self.child.poll() is not None:
                    raise ValueError(f"owned client PID {self.child.pid} exited {self.child.returncode}")
                continue
            block = os.read(self.child.stdout.fileno(), 65536)
            if not block:
                raise ValueError(f"owned client PID {self.child.pid} closed its result channel")
            self.buffer += block
            if len(self.buffer) > 131072:
                raise ValueError("client response exceeds 128 KiB")
        line, self.buffer = self.buffer.split(b"\n", 1)
        result = json.loads(line)
        if not self.pending or result.get("seq") != self.seq or not isinstance(result.get("result"), dict):
            raise ValueError("unexpected client command result")
        self.seq += 1
        self.pending = False
        return result["result"]

    def request(self, action, timeout, **args):
        self.send(action, **args)
        return self.receive(timeout)


class ApplicationTrial(Trial):
    def __init__(self, *args, mode="native", rounds=2, case_timeout=120, nfs_mount=None, **kwargs):
        super().__init__("g3", *args, **kwargs)
        self.mode, self.rounds, self.case_timeout = mode, rounds, case_timeout
        self.nfs_mount = canonical(nfs_mount) if nfs_mount else None
        self.clients, self.contenders, self.sequential = {}, [], []
        self.sequential_histories = []
        self.round_results, self.cold_ids = [], set()
        self.round_dir = None
        self.ganesha = None
        self.ganesha_identity = None
        self.ganesha_stable = None
        self.ganesha_log_offset = 0
        self.ganesha_log_inode = None
        self.capacity = None
        self.profile = "native" if mode == "native" else "nfs41"
        self.cases = selected_cases(self.profile)

    def preflight(self):
        if self.mode not in ("native", "ganesha-only", "mds-ganesha"):
            raise Prerequisite("unsupported application gate")
        if not 1 <= self.rounds <= 8 or not 1 <= self.case_timeout <= 300:
            raise Prerequisite("rounds must be 1..8 and case timeout 1..300 seconds")
        if self.mode == "native" and self.fuse_mount is None:
            raise Prerequisite("native application gate requires --fuse-mount")
        if self.mode != "native" and (self.nfs_mount is None or self.fuse_mount is not None):
            raise Prerequisite("Ganesha gate requires --nfs-mount and no --fuse-mount")
        if self.mode == "ganesha-only" and self.takeover_pid:
            raise Prerequisite("Ganesha-only restart cannot select an MDS takeover")
        required = ["ceph", "rados", "findmnt", "mount", "umount"]
        required += ["ceph-fuse"] if self.mode == "native" else ["ganesha.nfsd", "mount.nfs"]
        missing = [tool for tool in required if shutil.which(tool) is None]
        if missing:
            raise Prerequisite("missing required tools: " + ", ".join(missing))
        super().preflight()
        if self.takeover_pid:
            argv = process_identity(self.takeover_pid)["argv"]
            if "--keyring" not in argv or canonical(argv[argv.index("--keyring") + 1]) != self.keyring:
                raise Prerequisite("--keyring must also select the cold standby's subsequent restart identity")
        if self.nfs_mount:
            if (Path(os.environ["DEVCLUSTER_DATA"]) / "fsid").read_text().strip() != self.identity["fsid"]:
                raise Prerequisite("Ganesha helper data directory belongs to another cluster")
            require_unmounted(self.nfs_mount)
            if (not self.nfs_mount.is_dir() or any(self.nfs_mount.iterdir()) or
                    self.nfs_mount in (self.mount, self.cold_mount)):
                raise Prerequisite("NFS target must be a distinct existing empty local directory")
            external_path(self.mount, self.nfs_mount / "guard")
            shim = Path(os.environ.get("GCTX_SHIM", "/missing-shim"))
            if not shim.is_file():
                raise Prerequisite("existing Ganesha gctx shim required; no automatic build")
            directory = Path(os.environ["DEVCLUSTER_DATA"]) / "ganesha-recovery"
            if any(path.is_symlink() for path in (directory, directory / "recov", directory / "ganesha.conf")):
                raise Prerequisite("Ganesha recovery paths must not alias another directory")
            if os.environ.get("GANESHA_CONFIG_FSID"):
                raise Prerequisite("offline Ganesha render override is not permitted in a live controller")
            rendered = command(["bash", str(ROOT / "devcluster/ganesha.sh"), "print-config", "recovery"], timeout=10)
            config = directory / "ganesha.conf"
            if config.exists() and config.read_text().strip() != rendered.strip():
                raise Prerequisite("existing Ganesha recovery configuration differs; preserve it")
            # Read-only socket table check. Do not probe/stop the system server.
            for table in ("/proc/net/tcp", "/proc/net/tcp6"):
                for line in Path(table).read_text().splitlines()[1:]:
                    fields = line.split()
                    if fields[3] == "0A" and int(fields[1].split(":")[1], 16) == 12049:
                        raise Prerequisite("test port 12049 is already owned; no helper will be stopped")
        self.capacity = self.capacity_check(self.rounds)
        if self.takeover_pid:
            argv = process_identity(self.takeover_pid)["argv"]
            name = argv[argv.index("-i") + 1] if "-i" in argv else argv[argv.index("--id") + 1]
            self.capacity["selected_standby_budget"] = self.capacity_check(self.rounds, daemon_name=name)

    def check_target(self):
        super().check_target()
        data = json.loads(command(["ceph", "-c", str(self.conf), "fs", "dump", "-f", "json"]))
        standbys = data.get("standbys", [])
        if self.takeover_pid:
            if len(standbys) != 1 or standbys[0]["gid"] != self.standby["gid"]:
                raise ValueError("selected cold standby changed before failure")
        elif standbys:
            raise ValueError("unselected standby can race the explicit restart")

    def capacity_check(self, remaining, daemon_name=None):
        daemon_name = daemon_name or self.active["name"]
        try:
            text = self.conf.read_bytes().decode("utf-8")
        except UnicodeError as error:
            raise Prerequisite("daemon configuration is not valid UTF-8") from error
        settings = persistence_settings(text, daemon_name)
        limit = settings["rmds_max_checkpoint_generations"]
        interval = settings["rmds_checkpoint_interval"]
        records = settings["rmds_checkpoint_records"]
        pools = json.loads(command(["ceph", "-c", str(self.conf), "osd", "lspools", "-f", "json"]))
        names = [pool["poolname"] for pool in pools if pool["poolnum"] == self.identity["metadata_pool"]]
        if len(names) != 1:
            raise Prerequisite("metadata pool name cannot be bound to the preserved pool ID")
        base = ["rados", "-c", str(self.conf), "-p", names[0]]
        objects = json.loads(command(base + ["--all", "ls", "-f", "json"]))
        generations = {obj["name"].rsplit(".", 1)[0] for obj in objects if obj["namespace"] == "" and
                       re.fullmatch(r"rmds\.checkpoint\.0\.\d+\.\d+\.(state|manifest)", obj["name"])}
        # The old default estimate was eight record-triggered checkpoints per
        # round at16 records each. Scale that128-record estimate and the time
        # estimate with the actual selected daemon's thresholds.
        planned = remaining * ((128 + records - 1) // records +
                               (self.active_timeout + self.case_timeout + interval - 1) // interval) + 4
        report = {"daemon_id": daemon_name, "retained_generations": len(generations), "configured_limit": limit,
                  "planned_additional_generation_budget": planned, "remaining_rounds": remaining,
                  "checkpoint_interval_seconds": interval, "checkpoint_record_threshold": records,
                  "per_round_record_estimate": 128,
                  "estimate_not_reservation": True,
                  "native_tail": command(base + ["getxattr", "rmds.wal.0.0", "rmds.tail"]).strip(),
                  "native_trim": command(base + ["getxattr", "rmds.wal.0.0", "rmds.trimmed"]).strip(),
                  "checkpoint_objects": sorted(obj["name"] for obj in objects if
                                               obj["name"].rsplit(".", 1)[0] in generations)}
        if limit < 2 or len(generations) + planned >= limit:
            raise Prerequisite(f"checkpoint run budget insufficient: {report}; never delete metadata to continue")
        return report

    def assert_clients(self):
        self.check_mount()
        if self.fuse_mount:
            self.check_fuse()
        if self.nfs_mount:
            require_same_mount(self.nfs_mount, self.extra_mounts[self.nfs_mount])
        allowed = [(self.namespace, self.snapshot["id"])]
        allowed += [(self.namespace, row["id"]) for path, row in self.extra_mounts.items()
                    if row["fstype"] in ("ceph", "fuse.ceph-fuse")]
        self.audit_mounts(allowed)
        for client in self.clients.values():
            if client.child.poll() is not None:
                raise ValueError("continuously mounted client worker exited")

    def start_client(self, label, mount):
        with (self.out / f"{label}-stderr.log").open("xb") as log:
            child = subprocess.Popen([sys.executable, str(ROOT / "scripts/g3_client.py"), "client",
                                      "--mount", str(mount), "--history", str(self.out / f"{label}.jsonl"),
                                      "--label", label, "--profile", self.profile], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=log, start_new_session=True)
        self.clients[label] = ClientProcess(child)
        return self.clients[label].request("prepare", self.case_timeout)

    def start_contenders(self, prepared, mounts):
        for label, result in prepared.items():
            # Native contenders use the OTHER independent CephFS client.
            opposite = mounts["fuse" if label == "kernel" else "kernel"] if self.mode == "native" else mounts[label]
            for kind in ("fcntl", "flock"):
                history = self.out / f"contender-{label}-{kind}.jsonl"
                with (self.out / f"contender-{label}-{kind}.log").open("xb") as log:
                    child = subprocess.Popen([sys.executable, str(ROOT / "scripts/g3_client.py"), "contender",
                                              "--mount", str(opposite), "--history", str(history),
                                              "--root", result["manifest"]["root"], "--kind", kind,
                                              "--inode", str(result["locks"][kind]),
                                              "--root-inode", str(result["manifest"]["entries"][""]["ino"]),
                                              "--profile", self.profile], stdout=log,
                                             stderr=subprocess.STDOUT, start_new_session=True)
                self.contenders.append((label, kind, child, history))
        deadline = time.monotonic() + self.case_timeout
        while time.monotonic() < deadline:
            self.assert_locks_held()
            if all(any(row["event"] == "blocked" for row in events(path)) for _, _, _, path in self.contenders):
                return
            time.sleep(.01)
        raise TimeoutError("independent contenders did not prove initial lock exclusion")

    def assert_locks_held(self):
        for _, _, child, history in self.contenders:
            if child.poll() is not None or any(row["event"] in ("acquired", "early_grant") for row in events(history)):
                raise ValueError("lock contender exited or acquired rights before holder release")

    def start_sequential(self, mounts):
        self.sequential = []
        for label, mount in mounts.items():
            path = self.round_dir / f"sequential-{label}.jsonl"
            with (self.round_dir / f"sequential-{label}.log").open("xb") as log:
                child = subprocess.Popen([sys.executable, str(ROOT / "scripts/crash-history.py"), "record",
                                          "--mount", str(mount), "--history", str(path), "--pause-before", "2",
                                          "--profile", self.profile],
                                         stdin=subprocess.PIPE, stdout=log, stderr=subprocess.STDOUT,
                                         start_new_session=True)
            self.sequential.append((child, path))
            self.sequential_histories.append(path)
        deadline = time.monotonic() + self.case_timeout
        while time.monotonic() < deadline:
            if any(child.poll() is not None for child, _ in self.sequential):
                raise ValueError("sequential history exited before its client-intent barrier")
            logs = [events(path) for _, path in self.sequential]
            if all(rows and rows[-1].get("event") == "intent" and rows[-1].get("seq") == 2 for rows in logs):
                return
            time.sleep(.01)
        raise TimeoutError("sequential client-intent barrier deadline exceeded")

    def signal_mds(self):
        self.check_target()
        self.assert_clients()
        self.assert_locks_held()
        intent_at = time.monotonic_ns()
        write_json(self.round_dir / "stop-intent.json", {"pid": self.pid, "gid": self.active["gid"],
                                                       "signal": int(signal.SIGKILL), "monotonic_ns": intent_at})
        start = time.monotonic_ns()
        signal.pidfd_send_signal(self.pidfd, signal.SIGKILL)
        if self.mode == "mds-ganesha":
            second = self.signal_ganesha(wait=False)
            if second - start > 1_000_000_000:
                raise ValueError("combined failure signal skew exceeded1s; simultaneous case not established")
        write_json(self.round_dir / "stop-sent.json", {"pid": self.pid, "monotonic_ns": start})
        poller = select.poll()
        poller.register(self.pidfd, select.POLLIN)
        if not poller.poll(60000):
            raise TimeoutError("selected active MDS did not exit; no replacement is started")
        if self.mode == "mds-ganesha":
            self.ganesha.wait(timeout=60)
        return start

    def start_daemon(self):
        if self.takeover_pid:
            return super().start_daemon()
        if (hashlib.sha256(self.binary.read_bytes()).hexdigest() != self.binary_hash or
                hashlib.sha256(self.conf.read_bytes()).hexdigest() != self.conf_hash):
            raise ValueError("reviewed daemon binary/config changed")
        with (self.round_dir / "restart.log").open("x") as log:
            child = subprocess.Popen([str(self.binary), "-i", self.active["name"], "-c", str(self.conf),
                                      "--keyring", str(self.keyring)], stdout=log, stderr=subprocess.STDOUT,
                                     start_new_session=True)
        self.children.append(child)
        return child.pid

    def adopt_active(self, pid, active):
        fd = os.pidfd_open(pid)
        try:
            process = process_identity(pid)
            validate_target(process, active, self.conf)
            if hashlib.sha256(Path(f"/proc/{pid}/exe").read_bytes()).hexdigest() != self.binary_hash:
                raise ValueError("active process binary changed")
        except BaseException:
            os.close(fd)
            raise
        os.close(self.pidfd)
        self.pidfd, self.pid, self.active, self.process_start = fd, pid, active, process["start"]
        # A selected external cold standby is used for the first promotion.
        # Later rounds explicitly restart that promoted daemon ID/keyring.
        self.takeover_pid = None

    def mount_nfs(self, path):
        require_unmounted(path)
        command(["mount", "-t", "nfs", "-o", "vers=4.1,port=12049,nosharecache,hard,timeo=10,retrans=2",
                 "127.0.0.1:/a", str(path)], timeout=90)
        identity = capture_mount_identity(path)
        row = exact_physical_mount(path)
        options = dict(item.split("=", 1) for item in row["options"].split(",") if "=" in item)
        if (row["fstype"] not in ("nfs", "nfs4") or row["source"] != "127.0.0.1:/a" or
                options.get("port") != "12049" or options.get("vers") != "4.1" or
                identity["boot_id"] != self.snapshot["boot_id"]):
            raise ValueError("owned NFS mount identity/port not established")
        return identity

    def ganesha_fingerprint(self):
        directory = Path(os.environ["DEVCLUSTER_DATA"]) / "ganesha-recovery"
        return {"config_sha256": hashlib.sha256((directory / "ganesha.conf").read_bytes()).hexdigest(),
                "recovery_root": str(directory / "recov"), "recovery_inode": (directory / "recov").stat().st_ino,
                "node": "rust-mds-devcluster", "port": 12049, "configured_grace_seconds": 90}

    def start_ganesha(self):
        log_path = Path(os.environ["DEVCLUSTER_DATA"]) / "log/ganesha-recovery.log"
        if self.ganesha_stable is None and log_path.exists():
            self.ganesha_log_offset = log_path.stat().st_size
            self.ganesha_log_inode = log_path.stat().st_ino
        with (self.round_dir / "ganesha-stdout.log").open("xb") as log:
            child = subprocess.Popen(["bash", str(ROOT / "devcluster/ganesha.sh"), "run", "recovery"],
                                     stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        self.children.append(child)
        self.ganesha = child
        deadline = time.monotonic() + 60
        expected_conf = canonical(Path(os.environ["DEVCLUSTER_DATA"]) / "ganesha-recovery/ganesha.conf")
        while time.monotonic() < deadline:
            if child.poll() is not None:
                raise ValueError("owned recovery Ganesha exited; inspect its log")
            process = process_identity(child.pid)
            if process["exe"] == "ganesha.nfsd" and "127.0.0.1:12049" in process["listeners"]:
                argv = process["argv"]
                if "-f" not in argv or canonical(argv[argv.index("-f") + 1]) != expected_conf:
                    raise ValueError("Ganesha config ownership differs")
                stable = self.ganesha_fingerprint()
                if self.ganesha_stable is not None and stable != self.ganesha_stable:
                    raise ValueError("Ganesha stable identity/recovery directory changed")
                self.ganesha_stable = stable
                self.ganesha_identity = process
                write_json(self.round_dir / "ganesha-identity.json", {"pid": child.pid, "start": process["start"], **stable})
                return
            time.sleep(.1)
        raise TimeoutError("owned Ganesha did not bind test port12049")

    def capture_ganesha_evidence(self, label):
        """Retain bounded raw evidence, not inferred RESET or NFS reclaim success."""
        stable = self.ganesha_fingerprint()
        if stable != self.ganesha_stable:
            raise ValueError("Ganesha stable identity changed while capturing evidence")
        path = Path(os.environ["DEVCLUSTER_DATA"]) / "log/ganesha-recovery.log"
        st = path.stat()
        if ((self.ganesha_log_inode is not None and st.st_ino != self.ganesha_log_inode) or
                st.st_size < self.ganesha_log_offset):
            raise ValueError("Ganesha log rotated/truncated; complete phase evidence unavailable")
        with path.open("rb") as log:
            log.seek(self.ganesha_log_offset)
            data = log.read(2 * 1024 * 1024 + 1)
        if len(data) > 2 * 1024 * 1024:
            raise ValueError("Ganesha phase log exceeds2MiB; retain original and stop")
        (self.round_dir / f"ganesha-{label}.log").write_bytes(data)
        self.ganesha_log_inode, self.ganesha_log_offset = st.st_ino, self.ganesha_log_offset + len(data)
        entries, total = [], 0

        def collect(directory):
            nonlocal total
            with os.scandir(directory) as children:
                names = sorted(entry.name for entry in children)
            for name in names:
                item = directory / name
                mode = item.lstat().st_mode
                if not (stat.S_ISDIR(mode) or stat.S_ISREG(mode)):
                    raise ValueError("unexpected symlink/special file in owned Ganesha recovery state")
                size = item.lstat().st_size
                total += size
                if len(entries) >= 512 or size > 65536 or total > 2 * 1024 * 1024:
                    raise ValueError("Ganesha recovery evidence exceeds planned budget")
                entries.append({"path": str(item.relative_to(stable["recovery_root"])), "size": size,
                                "sha256": hashlib.sha256(item.read_bytes()).hexdigest() if stat.S_ISREG(mode) else None})
                if stat.S_ISDIR(mode):
                    collect(item)

        collect(Path(stable["recovery_root"]))
        write_json(self.round_dir / f"ganesha-{label}.json", {"stable": stable, "recovery_entries": entries,
                                                             "RESET_shared_session_and_NFS_reclaim_proven": False})

    def signal_ganesha(self, wait=True):
        child = self.ganesha
        if child is None or child.poll() is not None:
            raise ValueError("owned Ganesha is absent")
        fd = os.pidfd_open(child.pid)
        try:
            process = process_identity(child.pid)
            if (process["start"] != self.ganesha_identity["start"] or process["exe"] != "ganesha.nfsd" or
                    "127.0.0.1:12049" not in process["listeners"] or self.ganesha_fingerprint() != self.ganesha_stable):
                raise ValueError("Ganesha process/stable identity changed; no signal")
            stamp = time.monotonic_ns()
            signal.pidfd_send_signal(fd, signal.SIGKILL)
            if wait:
                child.wait(timeout=60)
            write_json(self.round_dir / "ganesha-stop.json", {"pid": child.pid, "monotonic_ns": stamp,
                                                             "signal": int(signal.SIGKILL)})
            return stamp
        finally:
            os.close(fd)

    def verify_at(self, mount, labels):
        for label in labels:
            output = command([sys.executable, str(ROOT / "scripts/g3_client.py"), "verify", "--mount", str(mount),
                              "--history", str(self.out / f"{label}.jsonl"), "--profile", self.profile], timeout=self.case_timeout)
            if json.loads(output) != {"verified": True}:
                raise ValueError("application comparison did not return its checked success")
            oracle = "restart_history.py" if self.profile == "native" else "crash-history.py"
            args = [sys.executable, str(ROOT / "scripts" / oracle), "verify", "--mount", str(mount),
                    "--history", str(self.out / f"mixed-{label}.jsonl")]
            if self.profile != "native":
                args += ["--profile", self.profile]
            command(args, timeout=self.case_timeout)
        for history in self.sequential_histories:
            # Complete acknowledged histories must exit0. A valid uncertain
            # prefix exits77 and therefore cannot satisfy this application gate.
            rows = events(history)
            if not rows or rows[0].get("version") != 2 or rows[0].get("profile") != self.profile:
                raise ValueError("application sequential history lacks the selected profile/root-identity contract")
            command([sys.executable, str(ROOT / "scripts/crash-history.py"), "verify", "--mount", str(mount),
                     "--history", str(history), "--profile", self.profile], timeout=self.case_timeout)

    def cold_compare(self, labels):
        self.assert_clients()
        known = {(row["boot_id"], row["unique_id"]) for row in [self.snapshot, *self.extra_mounts.values()]}
        captured = self.mount_kernel(self.cold_mount) if self.mode == "native" else self.mount_nfs(self.cold_mount)
        self.extra_mounts[self.cold_mount] = captured
        identity = (captured["boot_id"], captured["unique_id"])
        if identity in known or identity in self.cold_ids:
            raise ValueError("independent cold client retained a previous unique identity")
        self.cold_ids.add(identity)
        self.verify_at(self.cold_mount, labels)
        require_same_mount(self.cold_mount, captured)
        write_json(self.round_dir / "cold-client.json", captured)
        command(["umount", "--", str(self.cold_mount)], timeout=60)
        require_unmounted(self.cold_mount)
        del self.extra_mounts[self.cold_mount]

    def finish_locks(self):
        if sorted((label, kind) for label, kind, _, _ in self.contenders) != sorted(
                (label, kind) for label in self.clients for kind in ("fcntl", "flock")):
            raise ValueError("a selected independent lock contender is missing or duplicated")
        releases = {}
        for label, client in self.clients.items():
            releases[label] = time.monotonic_ns()
            client.request("release", self.case_timeout)
        proof = []
        for label, kind, child, path in self.contenders:
            if child.wait(timeout=self.case_timeout) != 0:
                raise ValueError("lock contender failed after release")
            rows = events(path)
            if ([row["event"] for row in rows] != ["intent", "blocked", "acquired"] or
                    rows[-1]["monotonic_ns"] < releases[label]):
                raise ValueError("lock reconstruction/release proof incomplete")
            proof.append({"client": label, "kind": kind, "release_intent_ns": releases[label],
                          "acquired_ns": rows[-1]["monotonic_ns"]})
        write_json(self.out / "lock-proof.json", proof)
        for client in self.clients.values():
            if client.request("close", self.case_timeout) != {"closed": True} or client.child.wait(timeout=10) != 0:
                raise ValueError("persistent client did not close successfully")
            client.child.stdin.close()
            client.child.stdout.close()

    def execute(self):
        self.out.mkdir()
        self.artifacts_created = True
        write_json(self.out / "identity-before.json", self.identity)
        write_json(self.out / "plan.json", {"gate": self.mode, "rounds": self.rounds, "capacity": self.capacity,
                                           "selected_cases": list(self.cases), "binary_sha256": self.binary_hash,
                                           "profile": self.profile, "excluded_cases": excluded_cases(self.profile),
                                           "config_sha256": self.conf_hash, "initial_mount": self.snapshot,
                                           "original_pid": self.pid, "original_start": self.process_start,
                                           "original_gid": self.active["gid"], "namespace": self.namespace,
                                           "selected_first_takeover": self.standby})
        self.workload_started = True
        self.phase = "preparing_clients"
        self.round_dir = self.out / "setup"
        self.round_dir.mkdir()
        if self.mode == "native":
            self.start_fuse()
            mounts = {"kernel": self.mount, "fuse": self.fuse_mount}
        else:
            self.start_ganesha()
            self.extra_mounts[self.nfs_mount] = self.mount_nfs(self.nfs_mount)
            mounts = {"nfs-a": self.nfs_mount, "nfs-b": self.nfs_mount}
        prepared = {label: self.start_client(label, mount) for label, mount in mounts.items()}
        if set(self.clients) != set(mounts):
            raise ValueError("a selected persistent client was not installed")
        self.start_contenders(prepared, mounts)
        for number in range(1, self.rounds + 1):
            self.round_dir = self.out / f"round-{number:03d}"
            self.round_dir.mkdir()
            self.phase = f"round{number}:arming"
            budget = self.capacity_check(self.rounds - number + 1)
            write_json(self.round_dir / "capacity.json", budget)
            self.assert_clients()
            self.check_target()
            self.assert_locks_held()
            for client in self.clients.values():
                result = client.request("arm", self.case_timeout, round=number)
                if result.get("durability") != "write-return-only" or result.get("round") != number:
                    raise ValueError("dirty writer acknowledgement missing")
            self.start_sequential(mounts)
            if self.mode != "native":
                self.capture_ganesha_evidence("before-failure")
            self.phase = f"round{number}:failure"
            self.assert_clients()
            self.assert_locks_held()
            if self.mode == "ganesha-only":
                start = self.signal_ganesha()
                active_at = None
            else:
                # For the combined case, send both signals before waiting for
                # recovery. Exact simultaneity is not claimed.
                start = self.signal_mds()
            for child, _ in self.sequential:
                child.stdin.write(b"continue\n")
                child.stdin.flush()
                child.stdin.close()
            for client in self.clients.values():
                client.send("recover", round=number)
            self.phase = f"round{number}:recovery"
            if self.mode != "ganesha-only":
                recovered_pid = self.start_daemon()
                active_at, identity, active = self.wait_active(recovered_pid)
                self.adopt_active(recovered_pid, active)
                write_json(self.round_dir / "recovered-active.json", {"pid": self.pid, "gid": active["gid"],
                                                                      "identity": identity})
            if self.mode != "native":
                self.start_ganesha()
            self.check_target()
            self.assert_locks_held()
            results = {label: client.receive(self.case_timeout) for label, client in self.clients.items()}
            for result in results.values():
                require_assertions(result, self.cases)
                if result.get("round") != number or result.get("first_success_monotonic_ns", 0) < start:
                    raise ValueError("post-failure progress timestamp/round invalid")
            for child, _ in self.sequential:
                if child.wait(timeout=self.case_timeout) != 0:
                    raise ValueError("sequential acknowledged history failed")
            self.phase = f"round{number}:comparison"
            self.assert_clients()
            for mount in set(mounts.values()):
                self.verify_at(mount, mounts)
            self.cold_compare(mounts)
            if self.mode != "native":
                self.capture_ganesha_evidence("after-recovery")
            self.assert_locks_held()
            progress = {label: (result["first_success_monotonic_ns"] - start) / 1e9 for label, result in results.items()}
            report = {"round": number, "client_assertions": {label: result["assertions"] for label, result in results.items()},
                      "continuous_mount_identities": True, "cold_comparison": True, "sequential_complete": True,
                      "no_early_lock_grant": True, "active_seconds": (active_at - start) / 1e9 if active_at else None,
                      "verified_sequential_histories": [str(path.relative_to(self.out)) for path in self.sequential_histories],
                      "client_reconnect_progress_seconds": progress, "first_success_seconds": min(progress.values()),
                      "protocol_reconnect_seconds": None,
                      "progress_definition": "O_EXCL create/write/file-fsync/parent-fsync completion; not protocol OPEN timing"}
            write_json(self.round_dir / "assertions.json", report)
            self.round_results.append(report)
        self.assert_locks_held()
        self.finish_locks()
        if len(self.round_results) != self.rounds:
            raise ValueError("not all selected rounds completed")
        for report in self.round_results:
            if set(report["client_assertions"]) != set(mounts):
                raise ValueError("a selected client result is missing")
            for assertions in report["client_assertions"].values():
                require_assertions({"assertions": assertions}, self.cases)
            if not all(report.get(key) is True for key in ("continuous_mount_identities", "cold_comparison",
                                                          "sequential_complete", "no_early_lock_grant")):
                raise ValueError("selected round proof incomplete")
        scope_pass = self.mode == "native" and self.rounds >= 2
        result = {"gate": f"g3-{self.mode}", "result": "APPLICATION_PASS" if scope_pass else "PARTIAL",
                  "coverage": "mounted-recovery-application" if self.mode == "native" else "nfs-mounted-application",
                  "profile": self.profile, "selected_cases": list(self.cases), "excluded_cases": excluded_cases(self.profile),
                  "identity": self.identity, "rounds": self.round_results, "full_g3_qualified": False,
                  "same_wire_tid_retry_proven": False, "flush_ack_loss_proven": False,
                  "client_intent_pause_not_wire_injection": True,
                  "unqualified": ["same-wire-tid-safe-reply-loss", "deterministic-FLUSH_ACK-loss",
                                  "protocol-reconnect-timing", "Ganesha-RESET/shared-session-flush-observation",
                                  "NFS-grace/reclaim-protocol", "NFS-unlink-open-server-restart",
                                  "native-CephFS-versus-NFS-lock-gap"],
                  "recovered_pid": self.pid, "ganesha_pid": self.ganesha.pid if self.ganesha else None}
        write_json(self.out / "client-state.json", {"primary": self.snapshot,
                                                    "extra_mounts": {str(path): row for path, row in self.extra_mounts.items()},
                                                    "fuse_process": self.fuse_process, "namespace": self.namespace})
        write_json(self.out / "result.json", result)
        self.phase = "complete"
        print(json.dumps(result))
        return 0 if scope_pass else 77

    def cleanup(self):
        # Close retained descriptors and lock waiters BEFORE normal unmount.
        for client in self.clients.values():
            self.stop_owned_child(client.child)
        for _, _, child, _ in self.contenders:
            self.stop_owned_child(child)
        for child, _ in self.sequential:
            self.stop_owned_child(child)
        if self.nfs_mount and exact_physical_mount(self.nfs_mount) is not None and self.nfs_mount not in self.extra_mounts:
            print("unvalidated NFS mount retained; retain helper and MDS for operator cleanup", file=sys.stderr)
            return
        super().cleanup()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("native", "ganesha-only", "mds-ganesha", "reply-loss", "flush-ack-loss"))
    parser.add_argument("pid", type=int)
    parser.add_argument("mount", type=Path)
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("--owned-mount", action="store_true")
    parser.add_argument("--rust-filesystem", action="store_true")
    parser.add_argument("--expected-fscid", required=True, type=int)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--keyring", required=True, type=Path)
    parser.add_argument("--cold-mount", required=True, type=Path)
    parser.add_argument("--fuse-mount", type=Path)
    parser.add_argument("--nfs-mount", type=Path)
    parser.add_argument("--takeover-pid", type=int)
    parser.add_argument("--rounds", type=int, default=2)
    parser.add_argument("--active-timeout", type=int, default=120)
    parser.add_argument("--case-timeout", type=int, default=120)
    args = parser.parse_args()
    if os.environ.get("RMDS_RECOVERY_READY") != "yes" or os.environ.get("RMDS_G3_READY") != "yes":
        print("DEPENDENCY: separate reviewed G3 readiness required; no live action")
        return 77
    if args.mode in ("reply-loss", "flush-ack-loss"):
        print("PREREQUISITE: deterministic owner/session/tid-tagged commit/send fault seam is not implemented", file=sys.stderr)
        return 2
    if args.mode != "native" and os.environ.get("RMDS_GANESHA_RECOVERY_READY") != "yes":
        print("DEPENDENCY: separate Ganesha recovery/runtime review required; no live action")
        return 77
    if os.geteuid() != 0:
        print("PREREQUISITE: root controller required", file=sys.stderr)
        return 2
    cap = int(next(line.split()[1] for line in Path("/proc/self/status").read_text().splitlines()
                   if line.startswith("CapEff:")), 16)
    if cap & ((1 << 21) | (1 << 5)) != ((1 << 21) | (1 << 5)):
        print("PREREQUISITE: CAP_SYS_ADMIN and CAP_KILL required", file=sys.stderr)
        return 2
    if not 1 <= args.active_timeout <= 600:
        parser.error("active timeout must be 1..600 seconds")
    trial = ApplicationTrial(args.pid, args.mount, args.artifacts, os.environ["CEPH_CONF"], [], args.owned_mount,
                             mode=args.mode, rounds=args.rounds, case_timeout=args.case_timeout, nfs_mount=args.nfs_mount,
                             binary=args.binary, keyring=args.keyring, rust_filesystem=args.rust_filesystem,
                             takeover_pid=args.takeover_pid, cold_mount=args.cold_mount, fuse_mount=args.fuse_mount,
                             active_timeout=args.active_timeout, expected_fscid=args.expected_fscid)

    def cancelled(signum, frame):
        raise Cancelled(f"controller cancelled by signal {signum}")

    signal.signal(signal.SIGTERM, cancelled)
    signal.signal(signal.SIGINT, cancelled)
    try:
        return trial.run()
    except Exception as error:
        code = failure_code(error, trial)
        print(f"ERROR: {error}", file=sys.stderr)
        if trial.artifacts_created:
            try:
                write_json(trial.out / "error.json", {"exit": code, "phase": trial.phase, "error": str(error),
                                                     "stdout": getattr(error, "output", None),
                                                     "stderr": getattr(error, "stderr", None),
                                                     "active_pid": trial.pid, "namespace": trial.namespace,
                                                     "primary": trial.snapshot,
                                                     "extra_mounts": {str(path): row for path, row in trial.extra_mounts.items()},
                                                     "owned_children": [child.pid for child in trial.children]})
            except OSError as artifact_error:
                print(f"cannot retain error.json: {artifact_error}", file=sys.stderr)
        return code
    finally:
        if trial.pidfd is not None:
            os.close(trial.pidfd)


if __name__ == "__main__":
    sys.exit(main())
