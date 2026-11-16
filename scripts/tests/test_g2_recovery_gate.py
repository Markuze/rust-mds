"""G2/G3 controller phase tests. Kernel/daemon boundaries are injected, not live claims."""
import importlib.util
import contextlib
import errno
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, call, patch
from oracle_faults import scandir_with_type_error

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("recovery_gate", ROOT / "scripts/recovery_trial.py")
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)
from mount_guard import ceph_mountinfo
ORACLE_SPEC = importlib.util.spec_from_file_location("gate_restart_history", ROOT / "scripts/restart_history.py")
oracle = importlib.util.module_from_spec(ORACLE_SPEC)
ORACLE_SPEC.loader.exec_module(oracle)

IDENTITY = {"fsid": "test-fsid", "name": "a", "fscid": 44, "metadata_pool": 88, "data_pools": [89]}
ACTIVE = {"gid": 101, "name": "rust", "rank": 0, "state": "up:active",
          "addrs": {"addrvec": [{"addr": "127.0.0.1:7401"}]}}
BOOT_ID = "00000000-0000-4000-8000-000000000001"


class GateTests(unittest.TestCase):
    def setUp(self):
        Path("/tmp/opencode").mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir="/tmp/opencode")
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.trial = module.Trial("g2", 9000, root / "mount", root / "artifacts", "/conf", [], True,
                                  rust_filesystem=True)
        self.trial.identity, self.trial.active = IDENTITY, ACTIVE
        self.trial.original_mount_id = 101
        self.trial.original_mount_identity = {"id": 101, "unique_id": 10001, "boot_id": BOOT_ID}
        self.trial.snapshot = dict(self.trial.original_mount_identity)
        self.trial.pidfd = 77

    def boundary_patches(self, *, remount_id=202, unique_id=10002, boot_id=BOOT_ID,
                         allocate_error=None, wait_error=None):
        def history(mode, *args, **kwargs):
            if mode == "allocate":
                if allocate_error:
                    raise allocate_error
                return {"future_allocations": 16, "first_success_monotonic_ns": 300}
            return {}

        def unmount():
            self.trial.snapshot = None

        # These explicit boundaries allow observer/order tests without mounting,
        # executing a pretend ceph-rmds binary, or hiding commands on PATH.
        patches = [patch.object(self.trial, "check_mount"), patch.object(self.trial, "check_target"),
                   patch.object(self.trial, "history", side_effect=history),
                   patch.object(self.trial, "unmount_clean", side_effect=unmount),
                   patch.object(module.signal, "pidfd_send_signal"),
                   patch.object(module.select, "poll"),
                   patch.object(self.trial, "start_daemon", return_value=9001),
                   patch.object(self.trial, "wait_active", side_effect=wait_error, return_value=(200, IDENTITY, {**ACTIVE, "gid": 102})),
                   patch.object(self.trial, "audit_mounts"),
                   patch.object(self.trial, "mount_kernel", return_value={"id": remount_id, "unique_id": unique_id, "boot_id": boot_id}),
                   patch.object(module.time, "monotonic_ns", side_effect=[100, 500])]
        mocks = [p.start() for p in patches]
        mocks[5].return_value.poll.return_value = [(77, 1)]
        for p in patches:
            self.addCleanup(p.stop)
        return mocks

    def test_g2_pass_requires_cold_remount_full_compare_and_nonreuse(self):
        mocks = self.boundary_patches()
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(self.trial.execute(), 0)
        mocks[4].assert_called_once_with(77, signal.SIGTERM)
        mocks[8].assert_called_once_with()
        mocks[9].assert_called_once_with(self.trial.mount)
        self.assertEqual(mocks[2].call_args_list,
                         [call("record"), call("verify"), call("allocate"), call("verify", future=True)])
        mocks[7].assert_called_once_with(9001)
        result = json.loads((self.trial.out / "result.json").read_text())
        self.assertEqual(result["result"], "PASS")
        self.assertEqual(result["identity"], IDENTITY)
        self.assertEqual(result["future_allocations"], 16)

    def test_same_unique_mount_never_passes_even_if_legacy_id_differs(self):
        mocks = self.boundary_patches(remount_id=202, unique_id=10001)
        with self.assertRaisesRegex(ValueError, "original unique mount identity"):
            self.trial.execute()
        self.assertEqual(mocks[2].call_args_list, [call("record")])
        self.assertFalse((self.trial.out / "result.json").exists())

    def test_recycled_legacy_id_passes_only_with_new_unique_identity(self):
        self.boundary_patches(remount_id=101, unique_id=10002)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(self.trial.execute(), 0)
        self.assertEqual(self.trial.snapshot["id"], self.trial.original_mount_id)
        self.assertNotEqual(self.trial.snapshot["unique_id"], self.trial.original_mount_identity["unique_id"])

    def test_changed_boot_id_cannot_establish_cold_remount(self):
        self.boundary_patches(boot_id="00000000-0000-4000-8000-000000000002")
        with self.assertRaisesRegex(ValueError, "boot identity changed"):
            self.trial.execute()
        self.assertFalse((self.trial.out / "result.json").exists())

    def test_allocator_or_partial_history_failure_cannot_pass(self):
        self.boundary_patches(allocate_error=ValueError("incomplete future history"))
        with self.assertRaisesRegex(ValueError, "incomplete future"):
            self.trial.execute()
        self.assertFalse((self.trial.out / "result.json").exists())

    def assert_enumeration_error_has_no_result(self, gate, type_error=False):
        self.trial.gate = gate
        self.trial.mount.mkdir()
        if gate == "g3":
            self.trial.cold_mount = self.trial.mount.parent / "cold"
        mocks = self.boundary_patches()
        observed = []

        def history(mode, mount=None, future=False):
            if mode == "record":
                with contextlib.redirect_stdout(io.StringIO()):
                    oracle.record(self.trial.mount, self.trial.out / "history.jsonl")
                return {}
            if mode == "allocate":
                return {"future_allocations": 16, "first_success_monotonic_ns": 300}
            if gate == "g3" and mount != self.trial.cold_mount:
                return {}
            # The controller's selected comparison runs the real shared oracle
            # on local data. Mount/restart boundaries alone remain modeled.
            manifest = oracle.load_manifest(self.trial.out / "history.jsonl")
            unreadable = self.trial.mount / manifest["root"] / "empty"
            (unreadable / "unexpected-resurrected-file").write_bytes(b"unexpected")
            observed.append((mode, mount, future))
            real_scandir = os.scandir
            def scandir(path):
                if Path(path) == unreadable:
                    raise OSError(errno.EIO, os.strerror(errno.EIO), str(path))
                return real_scandir(path)
            fault = scandir_with_type_error(real_scandir, unreadable) if type_error else scandir
            with patch.object(os, "scandir", side_effect=fault):
                oracle.verify_manifest(self.trial.mount, manifest)
            return {}

        mocks[2].side_effect = history
        stdout = io.StringIO()
        expected_error = ValueError if type_error else OSError
        with contextlib.redirect_stdout(stdout), self.assertRaises(expected_error) as error:
            self.trial.execute()
        if type_error:
            self.assertIn("directory enumeration incomplete", str(error.exception))
        else:
            self.assertEqual(error.exception.errno, errno.EIO)
        self.assertEqual(module.failure_code(error.exception, self.trial), 1)
        expected_mount = self.trial.cold_mount if gate == "g3" else None
        self.assertEqual(observed, [("verify", expected_mount, gate == "g3")])
        for name in ("result.json", "timings.json", "client-state.json"):
            self.assertFalse((self.trial.out / name).exists(), name)
        self.assertNotIn('"result": "PASS"', stdout.getvalue())
        self.assertNotIn('"result": "PARTIAL"', stdout.getvalue())

    def test_g2_cold_readdir_error_is_runtime_failure_without_pass_artifact(self):
        self.assert_enumeration_error_has_no_result("g2")

    def test_g3_cold_readdir_error_is_runtime_failure_without_partial_artifact(self):
        self.assert_enumeration_error_has_no_result("g3")

    def test_g2_type_lookup_error_is_runtime_failure_without_pass_artifact(self):
        self.assert_enumeration_error_has_no_result("g2", type_error=True)

    def test_g3_type_lookup_error_is_runtime_failure_without_partial_artifact(self):
        self.assert_enumeration_error_has_no_result("g3", type_error=True)

    def test_recovery_timeout_cannot_remount_or_pass(self):
        mocks = self.boundary_patches(wait_error=TimeoutError("not active"))
        with self.assertRaises(TimeoutError):
            self.trial.execute()
        mocks[9].assert_not_called()
        self.assertFalse((self.trial.out / "result.json").exists())

    def test_g3_keeps_primary_mounted_uses_cold_client_and_remains_partial(self):
        self.trial.gate = "g3"
        self.trial.cold_mount = self.trial.mount.parent / "cold"
        mocks = self.boundary_patches()
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(self.trial.execute(), 77)
        mocks[3].assert_not_called()
        mocks[4].assert_called_once_with(77, signal.SIGKILL)
        mocks[9].assert_called_once_with(self.trial.cold_mount)
        self.assertIn(call("verify", self.trial.cold_mount, future=True), mocks[2].call_args_list)
        result = json.loads((self.trial.out / "result.json").read_text())
        self.assertEqual(result["result"], "PARTIAL")
        self.assertGreater(result["timings"]["first_success_seconds"], result["timings"]["active_seconds"])

    def test_stale_old_active_entry_is_not_recovery(self):
        self.trial.active_timeout = 10
        new = {**ACTIVE, "gid": 102, "addrs": {"addrvec": [{"addr": "127.0.0.1:7402"}]}}
        process = {"exe": "ceph-rmds", "argv": ["ceph-rmds", "-c", "/conf", "-i", "rust"],
                   "listeners": {"127.0.0.1:7402"}}
        with patch.object(module, "cluster_state", side_effect=[(IDENTITY, ACTIVE), (IDENTITY, None), (IDENTITY, new)]) as state, \
                patch.object(module, "process_identity", return_value=process) as proc, \
                patch.object(module.hashlib, "sha256", return_value=Mock(hexdigest=lambda: None)), \
                patch.object(module.Path, "read_bytes", return_value=b"model binary"), \
                patch.object(module.time, "sleep"):
            self.trial.wait_active(9001)
            self.assertEqual(state.call_count, 3)
            proc.assert_called_once_with(9001)

    def test_exit_codes_do_not_classify_partial_execution_as_prerequisite(self):
        self.assertEqual(module.failure_code(module.Prerequisite("missing"), self.trial), 2)
        self.assertEqual(module.failure_code(TimeoutError("monitor"), self.trial), 1)
        self.trial.preflight_complete = True
        self.assertEqual(module.failure_code(BlockingIOError("lock busy"), self.trial), 2)
        self.trial.workload_started = True
        self.assertEqual(module.failure_code(module.Prerequisite("mount appeared"), self.trial), 1)
        self.assertEqual(module.failure_code(ValueError("partial history"), self.trial), 1)

    def test_help_describes_direct_launcher_without_live_probes(self):
        result = subprocess.run([sys.executable, str(ROOT / "scripts/recovery_trial.py"), "--help"],
                                env={**os.environ, "RMDS_RECOVERY_READY": "yes"},
                                text=True, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0)
        for flag in ("--expected-fscid", "--binary", "--cold-mount", "--takeover-pid", "--inflight"):
            self.assertIn(flag, result.stdout)

    def test_g2_readiness_does_not_enable_g3(self):
        env = {**os.environ, "RMDS_RECOVERY_READY": "yes"}
        env.pop("RMDS_G3_READY", None)
        result = subprocess.run([sys.executable, str(ROOT / "scripts/recovery_trial.py"), "g3", "0",
                                 str(self.trial.mount), str(self.trial.out), "--expected-fscid", "44",
                                 "--owned-mount", "--rust-filesystem"], env=env, text=True,
                                capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 77)
        self.assertIn("separate reviewed G3", result.stdout)
        self.assertFalse(self.trial.out.exists())

    def test_identity_change_is_not_a_delay_to_ignore(self):
        with patch.object(module, "cluster_state", return_value=({**IDENTITY, "fscid": 45}, None)):
            with self.assertRaisesRegex(ValueError, "identity changed"):
                self.trial.wait_active(9001)

    def test_foreign_mount_namespace_fails_cold_client_audit(self):
        rows = [{"namespace": "foreign", "id": 101, "target": "/other", "fstype": "ceph"}]
        with patch.object(module, "all_ceph_mounts", return_value=rows):
            with self.assertRaises(module.Prerequisite):
                self.trial.audit_mounts([(self.trial.namespace, 101)])

    def test_standby_for_another_filesystem_is_refused(self):
        self.trial.takeover_pid = 9002
        standby = {"gid": 102, "name": "follower", "state": "up:standby", "join_fscid": 45,
                   "addrs": {"addrvec": [{"addr": "127.0.0.1:7402"}]}}
        process = {"exe": "ceph-rmds", "argv": ["ceph-rmds", "-c", "/conf", "-i", "follower"],
                   "listeners": {"127.0.0.1:7402"}, "start": "123"}
        with patch.object(module, "process_identity", return_value=process), \
                patch.object(module, "command", return_value=json.dumps({"standbys": [standby]})):
            with self.assertRaisesRegex(module.Prerequisite, "another filesystem"):
                self.trial.check_standby()

    def test_standby_with_wrong_running_binary_is_refused(self):
        self.trial.takeover_pid = os.getpid()
        self.trial.binary_hash = "not-the-running-image-hash"
        standby = {"gid": 102, "name": "follower", "state": "up:standby", "join_fscid": 44,
                   "addrs": {"addrvec": [{"addr": "127.0.0.1:7402"}]}}
        process = {"exe": "ceph-rmds", "argv": ["ceph-rmds", "-c", "/conf", "-i", "follower"],
                   "listeners": {"127.0.0.1:7402"}, "start": "123"}
        with patch.object(module, "process_identity", return_value=process), \
                patch.object(module, "command", return_value=json.dumps({"standbys": [standby]})):
            with self.assertRaisesRegex(module.Prerequisite, "designated binary"):
                self.trial.check_standby()

    def test_mountinfo_decodes_spaces_without_touching_mounted_path(self):
        text = '101 1 0:99 / /mnt/test\\040space rw - ceph 127.0.0.1:/ rw\n'
        rows = ceph_mountinfo(text, "ns")
        self.assertEqual(rows, [{"namespace": "ns", "id": 101, "target": "/mnt/test space", "fstype": "ceph"}])

    def test_timeout_cancels_only_its_owned_command_session(self):
        args = [sys.executable, "-c", "unused mocked workload"]
        child = Mock(pid=9010)
        child.communicate.side_effect = [subprocess.TimeoutExpired(args, .05), ("", "")]
        with patch.object(module.subprocess, "Popen", return_value=child) as launch, \
                patch.object(module.os, "killpg") as killpg, \
                patch.object(module.signal, "pidfd_send_signal") as pid_signal:
            with self.assertRaisesRegex(TimeoutError, "owned command PID"):
                module.command(args, timeout=.05)
            launch.assert_called_once_with(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                           start_new_session=True)
            killpg.assert_called_once_with(child.pid, signal.SIGKILL)
            self.assertEqual(child.communicate.call_args_list, [call(timeout=.05), call(timeout=2)])
            child.stdout.close.assert_called_once_with()
            child.stderr.close.assert_called_once_with()
            pid_signal.assert_not_called()


if __name__ == "__main__":
    unittest.main()
