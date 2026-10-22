"""Read-only probes and state-machine regressions. No mount/signal commands run."""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "devcluster"))
import mount_guard as guard

SPEC = importlib.util.spec_from_file_location("recovery_trial", ROOT / "scripts/recovery_trial.py")
trial_module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(trial_module)

IDENTITY = {"fsid": "test-fsid", "name": "a", "fscid": 43, "metadata_pool": 86, "data_pools": [87]}
ACTIVE = {"gid": 101, "name": "rust", "rank": 0, "state": "up:active",
          "addrs": {"addrvec": [{"addr": "127.0.0.1:7401"}]}}


class SafetyTests(unittest.TestCase):
    def setUp(self):
        Path("/tmp/opencode").mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir="/tmp/opencode")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.mount = self.root / "mount with spaces"
        self.mount.mkdir()
        self.alias = self.root / "alias"
        self.alias.symlink_to(self.mount, target_is_directory=True)
        self.row = {"id": 101, "target": str(self.mount), "source": "127.0.0.1:6789:/",
                    "fstype": "ceph", "fsroot": "/", "maj:min": "0:99",
                    "options": "rw,fsid=test-fsid,mds_namespace=a"}

    def trial(self, owned=True):
        return trial_module.Trial("g3", os.getpid(), self.mount, self.root / "artifacts",
                                  "/not-a-ceph-config", ["never-executed"], owned)

    def test_mount_aliases_refuse_existing_mount(self):
        old = Path.cwd()
        os.chdir(self.root)
        try:
            aliases = [str(self.mount) + "/", str(self.mount) + "//", self.alias,
                       Path("mount with spaces"), Path("alias/../alias")]
            for path in aliases:
                with self.subTest(path=path), self.assertRaisesRegex(ValueError, "existing mount"):
                    guard.require_unmounted(path, [self.row])
        finally:
            os.chdir(old)

    def test_actual_root_mount_guard_is_read_only(self):
        self.assertIsNotNone(guard.exact_mount("/"))
        with self.assertRaisesRegex(ValueError, "existing mount"):
            guard.require_unmounted("/tmp/../")

    def test_smoke_refuses_real_mount_through_run_path_aliases_before_start(self):
        run = self.root / "run with spaces"
        run.mkdir()
        (run / "mnt-kernel").symlink_to("/", target_is_directory=True)
        alias = self.root / "run-alias"
        alias.symlink_to(run, target_is_directory=True)
        for spelling in (str(run) + "/", "run with spaces", str(alias), str(alias) + "//"):
            with self.subTest(spelling=spelling):
                result = subprocess.run(["bash", str(ROOT / "scripts/smoke.sh")], cwd=self.root,
                                        env={**os.environ, "RMDS_RUN_DIR": spelling,
                                             "RMDS_BIN": "/must-not-start-or-build"},
                                        text=True, capture_output=True, timeout=10)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("existing mount at /", result.stderr)
                self.assertFalse((run / "ceph-rmds.log").exists())
                self.assertFalse((run / "mnt-fuse").exists())
                self.assertEqual(os.readlink(run / "mnt-kernel"), "/")

    def test_stacked_mount_is_ambiguous(self):
        with self.assertRaisesRegex(ValueError, "stacked mounts"):
            guard.require_unmounted(self.alias, [self.row, dict(self.row, id=102)])

    def test_history_parent_symlink_inside_test_tree_is_rejected(self):
        outside = self.root / "outside"
        outside.mkdir()
        (outside / "into-tested-fs").symlink_to(self.mount, target_is_directory=True)
        history = outside / "into-tested-fs/history.jsonl"
        with self.assertRaisesRegex(ValueError, "outside the test mount"):
            guard.external_path(self.mount, history)
        self.assertFalse((self.mount / "history.jsonl").exists())

    def test_same_device_alias_is_rejected_even_outside_path(self):
        with self.assertRaisesRegex(ValueError, "different filesystem"):
            guard.external_path(self.mount, self.root / "history.jsonl")

    def test_history_final_symlink_does_not_bypass_exclusive_creation(self):
        history = self.root / "history.jsonl"
        target = self.root / "new-log"
        history.symlink_to(target)
        with self.assertRaisesRegex(ValueError, "itself be a symlink"):
            guard.external_path(self.mount, history)
        self.assertFalse(target.exists())

    def test_cli_history_rejects_real_root_before_writing(self):
        history = self.root / "history.jsonl"
        result = subprocess.run([sys.executable, str(ROOT / "scripts/crash-history.py"),
                                 "record", "--mount", "/", "--history", str(history)],
                                capture_output=True, text=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(history.exists())

    def test_explicit_mount_ownership_precedes_all_probes(self):
        trial = self.trial(owned=False)
        with patch.object(trial_module, "cluster_state") as probe, \
                patch.object(trial_module, "command") as commands:
            with self.assertRaisesRegex(ValueError, "owned-mount"):
                trial.run()
            trial.cleanup()
            probe.assert_not_called()
            commands.assert_not_called()
        self.assertFalse(trial.out.exists())

    def test_direct_python_entry_cannot_bypass_dependency_guard(self):
        env = dict(os.environ)
        env.pop("RMDS_RECOVERY_READY", None)
        out = self.root / "artifacts"
        result = subprocess.run([sys.executable, str(ROOT / "scripts/recovery_trial.py"),
                                 "g3", str(os.getpid()), str(self.mount), str(out),
                                 "--owned-mount", "--", "never-executed"],
                                env=env, text=True, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 77)
        self.assertFalse(out.exists())

    def test_failed_monitor_preflight_never_arms_cleanup(self):
        trial = self.trial()
        with patch.object(trial_module, "cluster_state", side_effect=TimeoutError("monitor")), \
                patch.object(trial_module, "command") as commands, \
                patch.object(trial_module.signal, "pidfd_send_signal") as send:
            with self.assertRaises(TimeoutError):
                trial.run()
            trial.cleanup()
            commands.assert_not_called()
            send.assert_not_called()
        self.assertIsNone(trial.snapshot)
        self.assertFalse(trial.out.exists())

    def test_wrong_filesystem_is_rejected_before_pid_or_workload(self):
        trial = self.trial()
        for row, fscid in [(dict(self.row, fstype="ext4"), 43),
                           (dict(self.row, options="fsid=other,mds_namespace=a"), 43),
                           (self.row, 42)]:
            with self.subTest(row=row, fscid=fscid), \
                    patch.object(trial_module, "cluster_state", return_value=(IDENTITY, ACTIVE)), \
                    patch.object(trial_module, "exact_mount", return_value=row), \
                    patch.object(trial_module, "mounted_fscid", return_value=fscid), \
                    patch.object(trial_module, "process_identity") as process, \
                    patch.object(trial, "history") as history, \
                    patch.object(trial_module.signal, "pidfd_send_signal") as send:
                with self.assertRaises(ValueError):
                    trial.run()
                trial.cleanup()
                process.assert_not_called()
                history.assert_not_called()
                send.assert_not_called()
                self.assertIsNone(trial.snapshot)
                self.assertFalse(trial.out.exists())

    def test_same_config_standby_target_is_rejected(self):
        process = {"exe": "ceph-rmds", "argv": ["ceph-rmds", "-c", "/conf", "-i", "rust"],
                   "listeners": {"127.0.0.1:7402"}}
        with self.assertRaisesRegex(ValueError, "standby refused"):
            trial_module.validate_target(process, ACTIVE, "/conf")
        process["listeners"] = {"127.0.0.1:7401"}
        trial_module.validate_target(process, ACTIVE, "/conf")
        process["exe"] = "ceph-osd"
        with self.assertRaisesRegex(ValueError, "not an MDS"):
            trial_module.validate_target(process, ACTIVE, "/conf")

    def test_wrong_pid_preflight_has_no_mutating_effects(self):
        trial = self.trial()
        with patch.object(trial_module, "cluster_state", return_value=(IDENTITY, ACTIVE)), \
                patch.object(trial_module, "exact_mount", return_value=self.row), \
                patch.object(trial_module, "mounted_fscid", return_value=43), \
                patch.object(trial, "history") as history, \
                patch.object(trial_module.os, "pidfd_open") as pidfd, \
                patch.object(trial_module.signal, "pidfd_send_signal") as send, \
                patch.object(trial_module, "command") as commands:
            # Probe the real test runner PID. It must never qualify as an MDS.
            with self.assertRaisesRegex(ValueError, "not an MDS"):
                trial.run()
            trial.cleanup()
            history.assert_not_called()
            pidfd.assert_not_called()
            send.assert_not_called()
            commands.assert_not_called()
        self.assertFalse(trial.out.exists())
        self.assertIsNone(trial.snapshot)

    def test_real_process_socket_probe_does_not_confuse_namespace_sockets(self):
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            listener.listen()
            process = trial_module.process_identity(os.getpid())
            self.assertIn(f"127.0.0.1:{listener.getsockname()[1]}", process["listeners"])
            with self.assertRaisesRegex(ValueError, "not an MDS"):
                trial_module.validate_target(process, ACTIVE, "/conf")

    def test_g2_unmount_failure_never_escalates_or_enables_cleanup(self):
        trial = self.trial()
        trial.snapshot = guard.mount_key(self.row)
        trial.workload_started = True
        with patch.object(trial, "check_mount"), \
                patch.object(trial_module, "command", side_effect=TimeoutError("busy mount")) as commands, \
                patch.object(trial_module.signal, "pidfd_send_signal") as send:
            with self.assertRaises(TimeoutError):
                trial.unmount_clean()
            trial.cleanup()
            commands.assert_called_once_with(["sudo", "umount", "--", str(self.mount)], timeout=60)
            send.assert_not_called()
        self.assertIsNone(trial.snapshot)

    def test_g2_execute_stops_at_failed_clean_unmount(self):
        # Exercise the post-preflight state machine, not a live successful probe.
        trial = self.trial()
        trial.gate = "g2"
        trial.identity = IDENTITY
        trial.snapshot = guard.mount_key(self.row)
        with patch.object(trial, "check_mount"), patch.object(trial, "check_target"), \
                patch.object(trial, "history") as history, \
                patch.object(trial_module, "command", side_effect=TimeoutError("busy mount")) as commands, \
                patch.object(trial_module.signal, "pidfd_send_signal") as send, \
                patch.object(trial, "start_daemon") as restart:
            with self.assertRaises(TimeoutError):
                trial.execute()
            trial.cleanup()
            history.assert_called_once_with("record")
            commands.assert_called_once_with(["sudo", "umount", "--", str(self.mount)], timeout=60)
            send.assert_not_called()
            restart.assert_not_called()

    def test_replaced_mount_is_never_cleaned_up(self):
        trial = self.trial()
        trial.snapshot = guard.mount_key(self.row)
        trial.workload_started = True
        with patch.object(trial_module, "exact_mount", return_value=dict(self.row, id=102)), \
                patch.object(trial_module, "command") as commands, \
                contextlib.redirect_stderr(io.StringIO()):
            trial.cleanup()
            commands.assert_not_called()


class GaneshaConfigTests(unittest.TestCase):
    def test_recovery_mode_is_separate_and_has_grace(self):
        configs = {}
        for mode in ("smoke", "recovery"):
            configs[mode] = subprocess.run(["bash", str(ROOT / "devcluster/ganesha.sh"), "print-config", mode],
                                            check=True, text=True, capture_output=True, timeout=10,
                                            env={**os.environ, "GANESHA_CONFIG_FSID": "00000000-0000-0000-0000-000000000001"}).stdout
        self.assertIn("Graceless = true;", configs["smoke"])
        self.assertIn("Graceless = false;", configs["recovery"])
        self.assertIn("ganesha-recovery/recov", configs["recovery"])
        self.assertIn("RecoveryBackend = fs;", configs["recovery"])
        self.assertIn("Grace_Period = 90;", configs["recovery"])
        self.assertIn("Lease_Lifetime = 60;", configs["recovery"])
        self.assertIn("NFS_Port = 12049;", configs["recovery"])
        self.assertIn("client_oc = false;", configs["recovery"])


if __name__ == "__main__":
    unittest.main()
