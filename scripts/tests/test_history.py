"""Offline tests for the external history oracle."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location(
    "crash_history", Path(__file__).parents[1] / "crash-history.py")
HISTORY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HISTORY)


class HistoryTests(unittest.TestCase):
    def setUp(self):
        Path("/tmp/opencode").mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir="/tmp/opencode")
        self.addCleanup(self.temp.cleanup)
        self.mount = Path(self.temp.name) / "mount"
        self.mount.mkdir()
        self.log = Path(self.temp.name) / "history.jsonl"
        HISTORY.record(self.mount, self.log)
        self.root = next(self.mount.iterdir())

    def test_complete_history(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(HISTORY.verify(self.mount, self.log), 0)

    def test_lost_delete(self):
        (self.root / "target").write_bytes(b"source")
        with self.assertRaisesRegex(AssertionError, "violates acknowledged"):
            HISTORY.verify(self.mount, self.log)

    def test_lost_overwrite(self):
        (self.root / "alias").write_bytes(b"target")
        with self.assertRaisesRegex(AssertionError, "violates acknowledged"):
            HISTORY.verify(self.mount, self.log)

    def test_incomplete_is_not_pass(self):
        lines = self.log.read_text().splitlines()
        self.log.write_text("\n".join(lines[:-1]) + "\n")
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(HISTORY.verify(self.mount, self.log), 77)

    def pending(self, seq):
        entries = [json.loads(line) for line in self.log.read_text().splitlines()]
        end = next(i for i, e in enumerate(entries) if e.get("event") == "intent" and e.get("seq") == seq)
        self.log.write_text("".join(json.dumps(e) + "\n" for e in entries[:end + 1]))

    def test_pending_unlink_allows_before_and_after(self):
        self.pending(5)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(HISTORY.verify(self.mount, self.log), 77)
            os.link(self.root / "alias", self.root / "target")
            self.assertEqual(HISTORY.verify(self.mount, self.log), 77)

    def test_pending_rename_allows_only_atomic_before_or_after(self):
        self.pending(2)
        (self.root / "alias").unlink()
        (self.root / "source").write_bytes(b"source")
        (self.root / "target").write_bytes(b"target")
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(HISTORY.verify(self.mount, self.log), 77)
            os.replace(self.root / "source", self.root / "target")
            self.assertEqual(HISTORY.verify(self.mount, self.log), 77)
        (self.root / "target").write_bytes(b"target")
        with self.assertRaisesRegex(AssertionError, "violates acknowledged"):
            HISTORY.verify(self.mount, self.log)

    def test_pending_create_allows_partial_data_but_not_arbitrary_data(self):
        self.pending(0)
        (self.root / "alias").unlink()
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(HISTORY.verify(self.mount, self.log), 77)
            (self.root / "source").write_bytes(b"")
            self.assertEqual(HISTORY.verify(self.mount, self.log), 77)
            (self.root / "source").write_bytes(b"sou")
            self.assertEqual(HISTORY.verify(self.mount, self.log), 77)
        (self.root / "source").write_bytes(b"wrong")
        with self.assertRaisesRegex(AssertionError, "violates acknowledged"):
            HISTORY.verify(self.mount, self.log)

    def test_ack_without_intent_is_rejected(self):
        entries = self.log.read_text().splitlines()
        entries.insert(2, json.dumps({"event": "ack", "seq": 0}))
        self.log.write_text("\n".join(entries) + "\n")
        with self.assertRaisesRegex(AssertionError, "ack without"):
            HISTORY.verify(self.mount, self.log)

    def test_acknowledged_create_cannot_have_partial_data(self):
        entries = [json.loads(line) for line in self.log.read_text().splitlines()]
        end = next(i for i, e in enumerate(entries) if e.get("event") == "ack" and e.get("seq") == 0)
        self.log.write_text("".join(json.dumps(e) + "\n" for e in entries[:end + 1]))
        (self.root / "alias").unlink()
        (self.root / "source").write_bytes(b"sou")
        with self.assertRaisesRegex(AssertionError, "violates acknowledged"):
            HISTORY.verify(self.mount, self.log)

    def test_pending_link_rejects_a_copy_instead_of_a_hardlink(self):
        self.pending(3)
        (self.root / "alias").unlink()
        (self.root / "target").write_bytes(b"source")
        (self.root / "alias").write_bytes(b"source")
        with self.assertRaisesRegex(AssertionError, "violates acknowledged"):
            HISTORY.verify(self.mount, self.log)
        (self.root / "alias").unlink()
        os.link(self.root / "target", self.root / "alias")
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(HISTORY.verify(self.mount, self.log), 77)


if __name__ == "__main__":
    unittest.main()
