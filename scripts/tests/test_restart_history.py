"""Filesystem-level oracle tests on local temporary data, not live Ceph evidence."""
import contextlib
import copy
import errno
import importlib.util
import io
import json
import os
from pathlib import Path
import runpy
import sys
import tempfile
import unittest
from unittest.mock import patch
from oracle_faults import scandir_with_type_error

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("restart_history", ROOT / "scripts/restart_history.py")
oracle = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(oracle)
import mount_guard


class RestartHistoryTests(unittest.TestCase):
    def setUp(self):
        Path("/tmp/opencode").mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir="/tmp/opencode")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.mount = self.base / "synthetic filesystem with 'quote and spaces"
        self.mount.mkdir()
        self.history = self.base / "oracle.jsonl"
        with contextlib.redirect_stdout(io.StringIO()):
            oracle.record(self.mount, self.history)
        self.manifest = oracle.load_manifest(self.history)
        self.root = self.mount / self.manifest["root"]

    def test_complete_mixed_workload(self):
        oracle.verify_manifest(self.mount, self.manifest)
        self.assertEqual(len(self.manifest["allocated"]), 10)
        self.assertEqual(self.manifest["entries"]["right/hard"]["ino"],
                         self.manifest["entries"]["right/replace"]["ino"])

    def test_readdir_error_cannot_hide_unexpected_child_in_empty_directory(self):
        unreadable = self.root / "empty"
        (unreadable / "unexpected-resurrected-file").write_bytes(b"unexpected")
        with self.assertRaisesRegex(ValueError, "namespace differs"):
            oracle.verify_manifest(self.mount, self.manifest)
        real_scandir = os.scandir
        for code in (errno.EIO, errno.EACCES):
            def scandir(path):
                if Path(path) == unreadable:
                    raise OSError(code, os.strerror(code), str(path))
                return real_scandir(path)
            with self.subTest(errno=code), patch.object(os, "scandir", side_effect=scandir), \
                    self.assertRaises(OSError) as error:
                oracle.verify_manifest(self.mount, self.manifest)
            self.assertEqual(error.exception.errno, code)

    def test_verification_command_returns_runtime_one_on_readdir_eio(self):
        unreadable = self.root / "empty"
        (unreadable / "unexpected-resurrected-file").write_bytes(b"unexpected")
        real_scandir = os.scandir
        def scandir(path):
            if Path(path) == unreadable:
                raise OSError(errno.EIO, os.strerror(errno.EIO), str(path))
            return real_scandir(path)
        # Supply only the local synthetic mount/isolation boundary. Keep real
        # history loading, directory traversal, CLI error handling and exit code.
        argv = [str(ROOT / "scripts/restart_history.py"), "verify", "--mount", str(self.mount),
                "--history", str(self.history)]
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(mount_guard, "exact_mount", return_value={"id": 1}), \
                patch.object(mount_guard, "external_path", return_value=self.history), \
                patch.object(os, "scandir", side_effect=scandir), patch.object(sys, "argv", argv), \
                contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr), \
                self.assertRaises(SystemExit) as exit_code:
            runpy.run_path(str(ROOT / "scripts/restart_history.py"), run_name="__main__")
        self.assertEqual(exit_code.exception.code, 1)
        self.assertNotIn('"verified": true', stdout.getvalue())
        self.assertIn("Input/output error", stderr.getvalue())

    def test_type_lookup_eio_cannot_skip_expected_directory_contents(self):
        directory = self.root / "empty"
        (directory / "unexpected-resurrected-file").write_bytes(b"unexpected")
        with self.assertRaisesRegex(ValueError, "namespace differs"):
            oracle.verify_manifest(self.mount, self.manifest)
        with patch.object(os, "scandir", side_effect=scandir_with_type_error(os.scandir, directory)), \
                self.assertRaisesRegex(ValueError, "directory enumeration incomplete"):
            oracle.verify_manifest(self.mount, self.manifest)

    def test_missing_visit_to_known_empty_directory_fails(self):
        # An expected empty directory is not verified merely by seeing its name
        # in its parent. Keep all other real enumeration and metadata checks.
        real_walk = os.walk
        for omitted in (self.root, self.root / "empty"):
            def walk(*args, **kwargs):
                for directory, dirs, files in real_walk(*args, **kwargs):
                    if Path(directory) != omitted:
                        yield directory, dirs, files
            with self.subTest(omitted=omitted), patch.object(os, "walk", side_effect=walk), \
                    self.assertRaisesRegex(ValueError, "directory enumeration incomplete"):
                oracle.verify_manifest(self.mount, self.manifest)

    def test_cli_type_lookup_eio_fails_without_verified_output(self):
        directory = self.root / "empty"
        (directory / "unexpected-resurrected-file").write_bytes(b"unexpected")
        argv = [str(ROOT / "scripts/restart_history.py"), "verify", "--mount", str(self.mount),
                "--history", str(self.history)]
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(mount_guard, "exact_mount", return_value={"id": 1}), \
                patch.object(mount_guard, "external_path", return_value=self.history), \
                patch.object(os, "scandir", side_effect=scandir_with_type_error(os.scandir, directory)), \
                patch.object(sys, "argv", argv), contextlib.redirect_stdout(stdout), \
                contextlib.redirect_stderr(stderr), self.assertRaises(SystemExit) as exit_code:
            runpy.run_path(str(ROOT / "scripts/restart_history.py"), run_name="__main__")
        self.assertEqual(exit_code.exception.code, 1)
        self.assertNotIn('"verified": true', stdout.getvalue())
        self.assertIn("directory enumeration incomplete", stderr.getvalue())

    def test_symlink_is_rejected_without_following_it(self):
        outside = self.base / "must-not-enumerate"
        outside.mkdir()
        (self.root / "empty" / "alias").symlink_to(outside, target_is_directory=True)
        real_scandir = os.scandir
        def scandir(path):
            self.assertNotEqual(Path(path), outside)
            return real_scandir(path)
        with patch.object(os, "scandir", side_effect=scandir), \
                self.assertRaisesRegex(ValueError, "unexpected symlink"):
            oracle.verify_manifest(self.mount, self.manifest)

    def test_acknowledged_deleted_file_cannot_reappear(self):
        (self.root / "deleted-file").write_bytes(b"delete-me")
        with self.assertRaisesRegex(ValueError, "namespace differs"):
            oracle.verify_manifest(self.mount, self.manifest)

    def test_acknowledged_deleted_directory_cannot_reappear(self):
        (self.root / "deleted-dir").mkdir()
        with self.assertRaisesRegex(ValueError, "namespace differs"):
            oracle.verify_manifest(self.mount, self.manifest)

    def test_overwritten_target_data_is_not_an_allowed_result(self):
        path = self.root / "right/replace"
        with path.open("r+b") as out:
            out.write(b"X")
        with self.assertRaisesRegex(ValueError, "data changed"):
            oracle.verify_manifest(self.mount, self.manifest)

    def test_copy_is_not_a_hard_link(self):
        path = self.root / "right/hard"
        path.unlink()
        path.write_bytes(oracle.PAYLOAD)
        with self.assertRaises(ValueError):
            oracle.verify_manifest(self.mount, self.manifest)

    def test_missing_xattr_fails(self):
        import os
        os.removexattr(self.root / "right/hard", "user.empty")
        with self.assertRaisesRegex(ValueError, "xattrs changed"):
            oracle.verify_manifest(self.mount, self.manifest)

    def test_partial_history_is_failure_not_pass(self):
        lines = self.history.read_text().splitlines()
        for remove in (1, 2, 15):
            with self.subTest(remove=remove):
                self.history.write_text("\n".join(lines[:-remove]) + "\n")
                with self.assertRaises(ValueError):
                    oracle.load_manifest(self.history)

    def test_deleted_and_overwritten_inodes_are_reserved_against_future_reuse(self):
        surviving = {entry["ino"] for entry in self.manifest["entries"].values()}
        deleted = set(self.manifest["allocated"]) - surviving
        self.assertEqual(len(deleted), 4)
        for ino in deleted:
            with self.assertRaisesRegex(ValueError, "reused"):
                oracle.fresh_inode(ino, self.manifest["allocated"])
        oracle.fresh_inode(max(self.manifest["allocated"]) + 1, self.manifest["allocated"])

    def test_complete_marker_cannot_hide_missing_steps(self):
        events = [json.loads(line) for line in self.history.read_text().splitlines()]
        final = dict(events[-1], steps=0)
        self.history.write_text(json.dumps(events[0]) + "\n" + json.dumps(final) + "\n")
        with self.assertRaisesRegex(ValueError, "every required operation"):
            oracle.load_manifest(self.history)

    def future_events(self):
        events = [{"event": "header", "purpose": "future-inode-nonreuse", "root": self.manifest["root"]}]
        for i in range(oracle.FUTURE_COUNT):
            ino = max(self.manifest["allocated"]) + i + 1
            entry = oracle.file_expected(ino, f"future-{i}".encode())
            entry["mtime_ns"] = 123
            events += [{"event": "intent", "seq": i, "create_write_fsync": f"left/future-{i:04d}"},
                       {"event": "ack", "seq": i, "ino": ino, "entry": entry, "monotonic_ns": 1}]
        events.append({"event": "complete", "count": oracle.FUTURE_COUNT, "first_success_monotonic_ns": 1})
        return events

    def test_future_history_requires_every_ack_and_reserves_old_ids(self):
        path = self.base / "future.jsonl"
        events = self.future_events()
        path.write_text("".join(json.dumps(e) + "\n" for e in events))
        merged = oracle.with_future(copy.deepcopy(self.manifest), path)
        self.assertEqual(len(merged["entries"]), len(self.manifest["entries"]) + oracle.FUTURE_COUNT)
        events[2]["ino"] = self.manifest["allocated"][0]
        path.write_text("".join(json.dumps(e) + "\n" for e in events))
        with self.assertRaisesRegex(ValueError, "reused"):
            oracle.with_future(copy.deepcopy(self.manifest), path)

    def test_partial_future_history_cannot_verify(self):
        path = self.base / "future.jsonl"
        path.write_text("".join(json.dumps(e) + "\n" for e in self.future_events()[:-1]))
        with self.assertRaisesRegex(ValueError, "incomplete future"):
            oracle.with_future(self.manifest, path)

    def test_failed_fsync_never_writes_a_completion_marker(self):
        history = self.base / "failed-fsync.jsonl"
        with patch.object(oracle, "sync_path", side_effect=OSError("flush failed")):
            with self.assertRaises(OSError):
                oracle.record(self.mount, history)
        self.assertNotIn('"event": "complete"', history.read_text())
        with self.assertRaises(ValueError):
            oracle.load_manifest(history)

    def test_manifest_cannot_omit_a_deleted_inode_from_ack_history(self):
        events = [json.loads(line) for line in self.history.read_text().splitlines()]
        events[-1]["manifest"]["allocated"][-1] = max(self.manifest["allocated"]) + 1
        self.history.write_text("".join(json.dumps(e) + "\n" for e in events))
        with self.assertRaisesRegex(ValueError, "lost acknowledged allocated"):
            oracle.load_manifest(self.history)


if __name__ == "__main__":
    unittest.main()
