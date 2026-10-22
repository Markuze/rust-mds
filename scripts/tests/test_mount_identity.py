"""Unique-mount parsing and complete controller paths. No mounts or signals run."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "devcluster"))
import mount_guard as guard

SPEC = importlib.util.spec_from_file_location("unique_mount_trial", ROOT / "scripts/recovery_trial.py")
controller = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(controller)

BOOT = "00000000-0000-4000-8000-000000000001"
IDENTITY = {"fsid": "test-fsid", "name": "a", "fscid": 44, "metadata_pool": 88, "data_pools": [89]}
ACTIVE = {"gid": 101, "name": "rust", "rank": 0, "state": "up:active",
          "addrs": {"addrvec": [{"addr": "127.0.0.1:7401"}]}}


class UniqueMountTests(unittest.TestCase):
    def setUp(self):
        Path("/tmp/opencode").mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir="/tmp/opencode")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.path = self.root / "mount with 'quote and spaces"
        self.path.mkdir()
        self.row = {"id": 323, "target": str(self.path), "source": "127.0.0.1:6789:/",
                    "fstype": "ceph", "fsroot": "/", "maj:min": "0:60",
                    "options": "fsid=test-fsid,mds_namespace=a"}

    def query(self, value=2147484001, target=None):
        return json.dumps({"filesystems": [{"target": target or str(self.path), "uniq-id": value}]})

    def test_capture_and_same_guard_resolve_aliases_and_keep_boot_scope(self):
        alias = self.root / "alias"
        alias.symlink_to(self.path, target_is_directory=True)
        with patch.object(guard, "mounts", return_value=[self.row]), \
                patch.object(guard, "boot_identity", return_value=BOOT), \
                patch.object(guard.subprocess, "run", return_value=Mock(stdout=self.query())) as query:
            for path in (str(self.path) + "/", self.path, alias, alias / ".." / "alias"):
                physical = guard.canonical(path)
                identity = guard.capture_mount_identity(physical)
                self.assertEqual(identity, {**guard.mount_key(self.row), "boot_id": BOOT, "unique_id": 2147484001})
                guard.require_same_mount(physical, identity)
            query.assert_called_with(["findmnt", "--kernel=listmount", "--json", "--mountpoint", str(self.path),
                                      "-o", "TARGET,UNIQ-ID"], check=True, text=True, capture_output=True, timeout=10)

    def test_malformed_unique_id_never_falls_back_to_legacy(self):
        payloads = [self.query(value) for value in (None, 0, -1, True, "2147484001", 1.5, 1 << 64)]
        payloads += ["not json", "{}", '{"filesystems":[]}', self.query(target="/another-mount"),
                     json.dumps({"filesystems": [{"target": str(self.path), "uniq-id": 1}] * 2}),
                     json.dumps({"filesystems": [{"target": str(self.path)}]})]
        for payload in payloads:
            with self.subTest(payload=payload), patch.object(guard, "mounts", return_value=[self.row]), \
                    patch.object(guard, "boot_identity", return_value=BOOT), \
                    patch.object(guard.subprocess, "run", return_value=Mock(stdout=payload)), \
                    self.assertRaises(guard.MountIdentityError):
                guard.capture_mount_identity(self.path)

    def test_unavailable_unique_interface_never_falls_back(self):
        errors = [FileNotFoundError("findmnt missing"), subprocess.CalledProcessError(1, ["findmnt"]),
                  subprocess.TimeoutExpired(["findmnt"], 10)]
        for error in errors:
            with self.subTest(error=type(error).__name__), patch.object(guard, "mounts", return_value=[self.row]), \
                    patch.object(guard, "boot_identity", return_value=BOOT), \
                    patch.object(guard.subprocess, "run", side_effect=error), \
                    self.assertRaises(guard.MountIdentityError):
                guard.capture_mount_identity(self.path)

    def test_invalid_or_changed_boot_identity_fails(self):
        for value in ("not-a-uuid", "", "00000000-0000-0000-0000-000000000000"):
            with self.subTest(value=value), patch.object(guard.Path, "read_text", return_value=value), \
                    self.assertRaises(ValueError):
                guard.boot_identity()
        with patch.object(guard, "mounts", return_value=[self.row]), \
                patch.object(guard, "boot_identity", side_effect=[BOOT, "00000000-0000-4000-8000-000000000002"]), \
                patch.object(guard.subprocess, "run", return_value=Mock(stdout=self.query())), \
                self.assertRaisesRegex(guard.MountIdentityError, "boot identity changed"):
            guard.capture_mount_identity(self.path)

    def test_same_legacy_metadata_with_changed_unique_or_boot_refuses_cleanup(self):
        captured = {**guard.mount_key(self.row), "unique_id": 2147484001, "boot_id": BOOT}
        for identity in (dict(captured, unique_id=2147484002),
                         dict(captured, boot_id="00000000-0000-4000-8000-000000000002")):
            with self.subTest(identity=identity), patch.object(guard, "capture_mount_identity", return_value=identity), \
                    self.assertRaisesRegex(guard.MountIdentityError, "refusing operation or cleanup"):
                guard.require_same_mount(self.path, captured)

    def test_replacement_during_capture_is_rejected(self):
        with patch.object(guard, "mounts", side_effect=[[self.row], [dict(self.row, id=324)]]), \
                patch.object(guard, "boot_identity", return_value=BOOT), \
                patch.object(guard.subprocess, "run", return_value=Mock(stdout=self.query())), \
                self.assertRaisesRegex(guard.MountIdentityError, "mount changed"):
            guard.capture_mount_identity(self.path)

    def test_primary_and_extra_cleanup_refuse_recycled_legacy_mount(self):
        captured = {**guard.mount_key(self.row), "unique_id": 2147484001, "boot_id": BOOT}
        for extra, unique, boot in ((False, 2147484002, BOOT), (True, 2147484002, BOOT),
                                   (False, 2147484001, "00000000-0000-4000-8000-000000000002"),
                                   (True, 2147484001, "00000000-0000-4000-8000-000000000002")):
            trial = controller.Trial("g2", 9000, self.path, self.root / "unused", "/conf", [], True)
            trial.workload_started = True
            trial.identity = IDENTITY
            trial.snapshot = None if extra else captured
            trial.extra_mounts = {self.path: captured} if extra else {}
            with self.subTest(extra=extra, unique=unique, boot=boot), patch.object(guard, "mounts", return_value=[self.row]), \
                    patch.object(guard, "boot_identity", return_value=boot), \
                    patch.object(guard.subprocess, "run", return_value=Mock(stdout=self.query(unique))), \
                    patch.object(controller, "command") as command, contextlib.redirect_stderr(io.StringIO()):
                trial.cleanup()
                command.assert_not_called()
            self.assertEqual(trial.extra_mounts[self.path] if extra else trial.snapshot, captured)

    def test_matching_unique_identity_allows_only_normal_owned_cleanup(self):
        captured = {**guard.mount_key(self.row), "unique_id": 2147484001, "boot_id": BOOT}
        for extra in (False, True):
            trial = controller.Trial("g2", 9000, self.path, self.root / "unused", "/conf", [], True)
            trial.workload_started = True
            trial.identity = IDENTITY
            trial.snapshot = None if extra else captured
            trial.extra_mounts = {self.path: captured} if extra else {}
            with self.subTest(extra=extra), patch.object(guard, "mounts", return_value=[self.row]), \
                    patch.object(guard, "boot_identity", return_value=BOOT), \
                    patch.object(guard.subprocess, "run", return_value=Mock(stdout=self.query())), \
                    patch.object(controller, "mounted_fscid", return_value=44), \
                    patch.object(controller, "command") as command:
                trial.cleanup()
                command.assert_called_once_with(["sudo", "umount", "--", str(self.path)], timeout=30)
            self.assertIsNone(trial.snapshot)
            self.assertEqual(trial.extra_mounts, {})


class CompleteControllerUniqueMountTests(unittest.TestCase):
    def setUp(self):
        Path("/tmp/opencode").mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir="/tmp/opencode")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def run_controller(self, *, cold_unique=2147484002, cold_legacy=323, preflight_payload=None,
                       preflight_error=None, runtime_payload=None, runtime_error=None):
        mount = self.root / "mount with 'quote and spaces"
        mount.mkdir(exist_ok=True)
        conf = self.root / "ceph.conf"
        conf.write_text("# synthetic read-only probe input\n")
        binary = self.root / "ceph-rmds"
        binary.write_bytes(b"not executed; process boundary is injected\n")
        binary.chmod(0o700)
        keyring = self.root / "keyring-placeholder"
        keyring.touch()
        trial = controller.Trial("g2", 9000, mount, self.root / "artifacts", conf, [], True,
                                 binary=binary, keyring=keyring, rust_filesystem=True, expected_fscid=44)
        initial = {"id": 323, "target": str(mount), "source": "127.0.0.1:6789:/", "fstype": "ceph",
                   "fsroot": "/", "maj:min": "0:60", "options": "fsid=test-fsid,mds_namespace=a"}
        state = {"mounted": True, "cold": False}
        audit_events, commands, oracle_steps = [], [], []

        def rows():
            return [dict(initial, id=cold_legacy if state["cold"] else 323)] if state["mounted"] else []

        def unique_query(*args, **kwargs):
            if runtime_error and state["cold"]:
                raise runtime_error
            if preflight_error and not state["cold"]:
                raise preflight_error
            payload = runtime_payload if state["cold"] else preflight_payload
            if payload is None:
                payload = json.dumps({"filesystems": [{"target": str(mount), "uniq-id": cold_unique if state["cold"] else 2147484001}]})
            return Mock(stdout=payload)

        def native_command(argv, timeout=20):
            commands.append(argv)
            if argv == ["ceph", "-c", str(conf), "fs", "dump", "-f", "json"]:
                return '{"standbys":[]}'
            if argv == ["sudo", "umount", "--", str(mount)]:
                state["mounted"] = False
                return ""
            if argv == [str(ROOT / "devcluster/mount-kernel.sh"), str(mount), "v1", "noshare"]:
                self.assertFalse(state["mounted"])
                state.update(mounted=True, cold=True)
                return ""
            raise AssertionError(f"unexpected native boundary: {argv}")

        def audit(allowed=()):
            audit_events.append((state["mounted"], list(allowed)))
            if not allowed:
                self.assertFalse(state["mounted"], "old mount exists at cold-client audit")

        def history(mode, *args, **kwargs):
            oracle_steps.append((mode, kwargs))
            return {"future_allocations": 16, "first_success_monotonic_ns": 300} if mode == "allocate" else {}

        def local_lock(path, mode):
            self.assertEqual(path, "/tmp/opencode/rust-mds-live.lock")
            return (self.root / "isolated-test-lane.lock").open(mode)

        process = {"exe": "ceph-rmds", "exe_path": str(binary), "start": "123",
                   "argv": [str(binary), "-i", "rust", "-c", str(conf)], "listeners": {"127.0.0.1:7401"}}
        stdout = io.StringIO()
        with contextlib.ExitStack() as stack:
            for boundary in [
                    patch.object(controller, "cluster_state", return_value=(IDENTITY, ACTIVE)),
                    patch.object(controller, "mounted_fscid", return_value=44),
                    patch.object(controller, "process_identity", return_value=process),
                    patch.object(controller, "external_path", return_value=trial.out),
                    patch.object(controller, "command", side_effect=native_command),
                    patch.object(guard, "mounts", side_effect=rows), patch.object(guard, "boot_identity", return_value=BOOT),
                    patch.object(guard.subprocess, "run", side_effect=unique_query),
                    patch.object(controller.fcntl, "flock"), patch.object(controller, "open", side_effect=local_lock, create=True),
                    patch.object(trial, "audit_mounts", side_effect=audit), patch.object(trial, "history", side_effect=history),
                    patch.object(trial, "wait_active", return_value=(200, IDENTITY, {**ACTIVE, "gid": 102})),
                    patch.object(controller.time, "monotonic_ns", side_effect=[100, 500]),
                    contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(io.StringIO())]:
                stack.enter_context(boundary)
            pidfd = stack.enter_context(patch.object(controller.os, "pidfd_open", return_value=77))
            send = stack.enter_context(patch.object(controller.signal, "pidfd_send_signal"))
            poll = stack.enter_context(patch.object(controller.select, "poll"))
            restart = stack.enter_context(patch.object(trial, "start_daemon", return_value=9001))
            poll.return_value.poll.return_value = [(77, 1)]
            try:
                code, error = trial.run(), None
            except Exception as exception:
                error = exception
                code = controller.failure_code(exception, trial)
        return trial, code, error, commands, oracle_steps, audit_events, send.call_count, restart.call_count, pidfd.call_count

    def test_complete_run_recycled_legacy_id_new_unique_reaches_future_proof(self):
        trial, code, error, commands, steps, audits, signals, restarts, _ = self.run_controller()
        self.assertEqual(code, 0)
        self.assertIsNone(error)
        self.assertEqual([mode for mode, _ in steps], ["record", "verify", "allocate", "verify"])
        self.assertEqual(steps[-1], ("verify", {"future": True}))
        self.assertEqual(signals, 1)
        self.assertEqual(restarts, 1)
        self.assertEqual(len([state for state, allowed in audits if not allowed and not state]), 2)
        self.assertIn([str(ROOT / "devcluster/mount-kernel.sh"), str(trial.mount), "v1", "noshare"], commands)
        before = json.loads((trial.out / "client-before.json").read_text())["primary"]
        after = json.loads((trial.out / "client-state.json").read_text())["primary"]
        self.assertEqual(before["id"], after["id"])
        self.assertEqual(before["boot_id"], after["boot_id"])
        self.assertNotEqual(before["unique_id"], after["unique_id"])
        self.assertEqual(json.loads((trial.out / "result.json").read_text())["result"], "PASS")

    def test_complete_run_same_unique_refuses_even_when_legacy_changes(self):
        trial, code, error, _, steps, _, _, _, _ = self.run_controller(cold_unique=2147484001, cold_legacy=324)
        self.assertEqual(code, 1)
        self.assertIn("original unique mount identity", str(error))
        self.assertEqual([mode for mode, _ in steps], ["record"])
        self.assertFalse((trial.out / "result.json").exists())

    def test_unavailable_unique_interface_is_preflight_exit_two_without_effects(self):
        trial, code, error, commands, steps, _, signals, restarts, pidfds = self.run_controller(
            preflight_error=subprocess.CalledProcessError(1, ["findmnt"]))
        self.assertEqual(code, 2)
        self.assertIsInstance(error, guard.MountIdentityError)
        self.assertEqual((commands, steps, signals, restarts, pidfds), ([], [], 0, 0, 0))
        self.assertFalse(trial.workload_started)
        self.assertIsNone(trial.snapshot)
        self.assertFalse(trial.out.exists())

    def test_malformed_unique_id_is_preflight_exit_two(self):
        trial, code, error, commands, steps, _, signals, restarts, _ = self.run_controller(preflight_payload='{"filesystems":[]}')
        self.assertEqual(code, 2)
        self.assertIsInstance(error, guard.MountIdentityError)
        self.assertEqual((commands, steps, signals, restarts), ([], [], 0, 0))
        self.assertFalse(trial.out.exists())

    def test_malformed_cold_unique_id_is_runtime_one_without_pass(self):
        trial, code, error, _, steps, _, _, _, _ = self.run_controller(runtime_payload='{"filesystems":[]}')
        self.assertEqual(code, 1)
        self.assertIsInstance(error, guard.MountIdentityError)
        self.assertEqual([mode for mode, _ in steps], ["record"])
        for name in ("result.json", "timings.json", "client-state.json"):
            self.assertFalse((trial.out / name).exists())

    def test_unavailable_cold_unique_interface_is_runtime_one(self):
        trial, code, error, _, steps, _, _, _, _ = self.run_controller(
            runtime_error=subprocess.CalledProcessError(1, ["findmnt"]))
        self.assertEqual(code, 1)
        self.assertIsInstance(error, guard.MountIdentityError)
        self.assertTrue(trial.workload_started)
        self.assertEqual([mode for mode, _ in steps], ["record"])
        self.assertFalse((trial.out / "result.json").exists())


if __name__ == "__main__":
    unittest.main()
