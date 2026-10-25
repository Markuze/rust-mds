"""Local filesystem oracles and injected recovery boundaries. Never live gates."""
import contextlib
import copy
import errno
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import g3_client as client
import g3_trial as gate
import recovery_trial as base

IDENTITY = {"fsid": "test-fsid", "name": "a", "fscid": 44, "metadata_pool": 88, "data_pools": [89]}
ACTIVE = {"gid": 101, "name": "rust", "rank": 0, "state": "up:active",
          "addrs": {"addrvec": [{"addr": "127.0.0.1:7401"}]}}


def replace_descriptor_proofs(rows, change):
    """Mutate real external evidence without changing the worker or its files."""
    count = 0
    for row in rows:
        objects = [row, row.get("result", {})]
        for obj in objects:
            for field in ("unlinked_before_write", "unlinked_fsync_ack", "unlinked_observation"):
                if field in obj:
                    obj[field] = change(copy.deepcopy(obj[field]))
                    count += 1
    return count


class LocalClientTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir="/tmp/opencode")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.mount = self.root / "mount 'with spaces"
        self.mount.mkdir()
        self.actor = client.Client(self.mount, self.root / "kernel.jsonl", "kernel")
        self.addCleanup(self.actor.close)

    def prepare(self):
        with contextlib.redirect_stdout(io.StringIO()):
            return self.actor.prepare()

    def test_real_mixed_symlink_truncate_dirty_and_unlinked_descriptor_checks(self):
        initial = self.prepare()
        inode = initial["unlinked_inode"]
        for round in (1, 2):
            armed = self.actor.arm(round)
            self.assertEqual(armed["durability"], "write-return-only")
            result = self.actor.recover(round)
            gate.require_assertions(result, client.CASES)
            self.assertEqual(result["unlinked_inode"], inode)
            client.compare(self.mount, result["manifest"])
        for name, target in (("link", "target"), ("dangling", "absent"), ("dir-link", "empty")):
            self.assertEqual(os.readlink(self.actor.root / name), target)
        self.assertFalse((self.actor.root / "unlinked").exists())

    def test_positive_symlink_changed_target_or_regular_substitution_fails(self):
        result = self.prepare()
        link = self.actor.root / "link"
        link.unlink()
        link.symlink_to("absent")
        with self.assertRaisesRegex(ValueError, "symlink|inode"):
            client.compare(self.mount, result["manifest"])
        link.unlink()
        link.write_bytes(client.PAYLOAD)
        with self.assertRaisesRegex(ValueError, "symlink|inode"):
            client.compare(self.mount, result["manifest"])

    def test_unexpected_symlink_is_not_followed_and_file_substitution_fails(self):
        result = self.prepare()
        outside = self.root / "outside"
        outside.mkdir()
        (outside / "not-in-test").write_bytes(b"outside")
        path = self.actor.root / "empty"
        path.rmdir()
        path.symlink_to(outside, target_is_directory=True)
        with self.assertRaises(ValueError):
            client.compare(self.mount, result["manifest"])
        self.assertEqual((outside / "not-in-test").read_bytes(), b"outside")

    def test_directory_io_error_is_not_empty_directory_success(self):
        result = self.prepare()
        real = os.scandir

        def fail_empty(path):
            physical = Path(os.readlink(f"/proc/self/fd/{path}")) if isinstance(path, int) else Path(path)
            if physical == self.actor.root / "empty":
                raise OSError(errno.EIO, "injected scandir failure")
            return real(path)

        with patch.object(client.os, "scandir", side_effect=fail_empty), self.assertRaises(OSError):
            client.compare(self.mount, result["manifest"])

    def test_changed_retained_inode_is_not_adopted_after_fsync(self):
        self.prepare()
        self.actor.arm(1)
        path = self.actor.root / "dirty"
        path.rename(self.actor.root / "old-dirty")
        path.write_bytes(self.actor.dirty_data)
        with self.assertRaisesRegex(ValueError, "retained inode changed"):
            self.actor.recover(1)

    def test_holder_comparison_does_not_close_an_extra_descriptor_and_drop_posix_lock(self):
        self.prepare()
        probe = """import errno,fcntl,os,sys
fd=os.open(sys.argv[1],os.O_RDWR)
try:
    fcntl.lockf(fd,fcntl.LOCK_EX|fcntl.LOCK_NB)
except OSError as error:
    sys.exit(0 if error.errno in (errno.EAGAIN,errno.EACCES) else 2)
else:
    sys.exit(1)
finally:
    os.close(fd)
"""
        for _ in range(2):
            self.actor.check()
            result = subprocess.run([sys.executable, "-c", probe, str(self.actor.root / "fcntl")],
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 0, "holder comparison released its POSIX lock")

    def test_cancelled_client_intent_uses_existing_uncertain_model_but_cannot_pass(self):
        spec = importlib.util.spec_from_file_location("g3_uncertain_history", ROOT / "scripts/crash-history.py")
        oracle = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(oracle)
        history = self.root / "uncertain.jsonl"
        with patch.object(oracle.sys, "stdin", io.StringIO("")), self.assertRaisesRegex(AssertionError, "barrier cancelled"):
            oracle.record(self.mount, history, pause_before=2)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(oracle.verify(self.mount, history), 77)

    def test_dirty_or_unacknowledged_client_history_cannot_be_cold_verified(self):
        log = self.root / "history.jsonl"
        commands = [{"seq": 0, "action": "prepare"}, {"seq": 1, "action": "arm", "args": {"round": 1}},
                    {"seq": 2, "action": "recover", "args": {"round": 1}}, {"seq": 3, "action": "close"}]
        with patch.object(client.sys, "stdin", io.StringIO("\n".join(map(json.dumps, commands)) + "\n")), \
                contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(self.actor.serve(), 0)
        rows = [json.loads(line) for line in self.actor.history.read_text().splitlines()]
        for armed_ack in (False, True):
            prefix = [row for row in rows if row.get("seq", -1) < 1 or row.get("seq") == 1 and
                      (armed_ack or row["event"] == "intent")]
            log.write_text("\n".join(map(json.dumps, prefix)) + "\n")
            with self.assertRaisesRegex(ValueError, "incomplete or unflushed"):
                client.load_snapshot(log)

    def assert_review_unlinked_fault_is_not_healed(self, fault):
        self.prepare()
        self.actor.arm(1)
        fd = self.actor.fds["unlinked"]
        if fault == "truncate":
            os.ftruncate(fd, 0)
        elif fault == "overwrite":
            os.pwrite(fd, b"!" * len(self.actor.unlinked_data), 0)
        elif fault == "extend":
            os.ftruncate(fd, len(self.actor.unlinked_data) + 9)
        before = os.pread(fd, os.fstat(fd).st_size + 1, 0)
        real_pread = os.pread

        def read(number, size, offset):
            if fault == "read-eio" and number == fd:
                raise OSError(errno.EIO, "injected surviving-descriptor read error")
            return real_pread(number, size, offset)

        with patch.object(client.os, "pwrite", wraps=os.pwrite) as write, \
                patch.object(client.os, "ftruncate", wraps=os.ftruncate) as truncate, \
                patch.object(client.os, "pread", side_effect=read):
            with self.assertRaises((ValueError, OSError)):
                self.actor.recover(1)
            self.assertFalse(any(call.args[0] == fd for call in write.call_args_list), "recovery healed saved bytes")
            self.assertFalse(any(call.args[0] == fd for call in truncate.call_args_list), "recovery healed saved size")
        self.assertEqual(real_pread(fd, os.fstat(fd).st_size + 1, 0), before)

    def test_review_unlinked_truncate_is_observed_before_repair(self):
        self.assert_review_unlinked_fault_is_not_healed("truncate")

    def test_review_unlinked_overwrite_is_observed_before_repair(self):
        self.assert_review_unlinked_fault_is_not_healed("overwrite")

    def test_review_unlinked_extension_is_observed_before_repair(self):
        self.assert_review_unlinked_fault_is_not_healed("extend")

    def test_review_unlinked_read_error_precedes_any_repair_write(self):
        self.assert_review_unlinked_fault_is_not_healed("read-eio")

    def assert_review_root_symlink_has_no_external_writes(self, clone=False):
        self.prepare()
        self.actor.arm(1)
        saved = self.mount / "saved-root"
        self.actor.root.rename(saved)
        outside = self.root / "external victim"
        outside.mkdir()
        if clone:
            shutil.copytree(saved, outside, symlinks=True, dirs_exist_ok=True)
        (outside / "sentinel").write_bytes(b"must stay unchanged")
        self.actor.root.symlink_to(outside, target_is_directory=True)

        def snapshot():
            return {str(path.relative_to(outside)): ("link", os.readlink(path)) if path.is_symlink()
                    else ("dir",) if path.is_dir() else ("file", path.read_bytes()) for path in outside.rglob("*")}

        before = snapshot()
        with self.assertRaises((ValueError, OSError)):
            self.actor.recover(1)
        self.assertEqual(snapshot(), before, "recovery wrote through a replaced root symlink")

    def test_review_recovery_root_symlink_cannot_write_outside_namespace(self):
        self.assert_review_root_symlink_has_no_external_writes()

    def test_review_lookalike_root_symlink_cannot_write_outside_namespace(self):
        self.assert_review_root_symlink_has_no_external_writes(clone=True)

    def test_review_next_round_validates_root_before_descriptor_mutation(self):
        self.prepare()
        outside = self.root / "external victim"
        outside.mkdir()
        (outside / "sentinel").write_bytes(b"unchanged")
        self.actor.root.rename(self.mount / "saved-root")
        self.actor.root.symlink_to(outside, target_is_directory=True)
        with patch.object(client.os, "ftruncate", wraps=os.ftruncate) as truncate, \
                patch.object(client.os, "pwrite", wraps=os.pwrite) as write:
            with self.assertRaises((ValueError, OSError)):
                self.actor.arm(1)
            truncate.assert_not_called()
            write.assert_not_called()
        self.assertEqual((outside / "sentinel").read_bytes(), b"unchanged")

    def test_review_prepare_rejects_replaced_mount_before_history_root_creation(self):
        self.mount.rename(self.root / "saved-mount")
        outside = self.root / "external victim"
        outside.mkdir()
        (outside / "sentinel").write_bytes(b"unchanged")
        self.mount.symlink_to(outside, target_is_directory=True)
        with self.assertRaises((ValueError, OSError)):
            self.prepare()
        self.assertEqual(sorted(path.name for path in outside.iterdir()), ["sentinel"])
        self.assertEqual((outside / "sentinel").read_bytes(), b"unchanged")

    def test_review_paused_sequential_history_root_does_not_follow_replacement(self):
        spec = importlib.util.spec_from_file_location("review_seq_root", ROOT / "scripts/crash-history.py")
        oracle = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(oracle)
        history = self.root / "paused-history.jsonl"
        outside = self.root / "external victim"
        outside.mkdir()
        (outside / "source").write_bytes(b"outside source")
        (outside / "target").write_bytes(b"outside target")

        def replace_at_barrier():
            root = self.mount / json.loads(history.read_text().splitlines()[0])["name"]
            root.rename(self.mount / "saved-sequential-root")
            root.symlink_to(outside, target_is_directory=True)
            return "continue\n"

        with patch.object(oracle.sys, "stdin", Mock(readline=replace_at_barrier)):
            with self.assertRaises((ValueError, OSError, AssertionError)):
                oracle.record(self.mount, history, pause_before=2)
        self.assertEqual({path.name: path.read_bytes() for path in outside.iterdir()},
                         {"source": b"outside source", "target": b"outside target"})

    def test_review_nfs41_declares_profile_without_unsupported_xattr_calls(self):
        actor = client.Client(self.mount, self.root / "nfs-a.jsonl", "nfs-a")
        self.addCleanup(actor.close)
        unsupported = OSError(errno.EOPNOTSUPP, "NFSv4.1 user xattrs unsupported")
        with patch.object(client.os, "setxattr", side_effect=unsupported) as set_attr, \
                patch.object(client.os, "getxattr", side_effect=unsupported) as get_attr, \
                patch.object(client.os, "listxattr", side_effect=unsupported) as list_attr:
            actor.prepare()
            actor.arm(1)
            result = actor.recover(1)
            self.assertIs(result["assertions"].get("xattrs"), False)
            self.assertIs(result["assertions"].get("nfs41_ack_history"), True)
            set_attr.assert_not_called()
            get_attr.assert_not_called()
            list_attr.assert_not_called()

    def test_native_xattrs_remain_mandatory_when_the_syscall_is_unsupported(self):
        with patch.object(client.os, "setxattr", side_effect=OSError(errno.EOPNOTSUPP, "unsupported")):
            with self.assertRaises(OSError):
                self.prepare()
        self.assertFalse(self.actor.prepared)

    def test_replaced_real_root_inode_is_rejected_before_new_mutation(self):
        self.prepare()
        self.actor.arm(1)
        saved = self.mount / "saved-root"
        self.actor.root.rename(saved)
        shutil.copytree(saved, self.actor.root, symlinks=True)
        before = sorted(path.name for path in self.actor.root.iterdir())
        with patch.object(client.os, "pwrite", wraps=os.pwrite) as write, \
                patch.object(client.os, "ftruncate", wraps=os.ftruncate) as truncate:
            with self.assertRaisesRegex(ValueError, "directory identity changed"):
                self.actor.recover(1)
            write.assert_not_called()
            truncate.assert_not_called()
        self.assertEqual(sorted(path.name for path in self.actor.root.iterdir()), before)

    def test_external_history_contains_fsynced_prewrite_observation_before_unlinked_write(self):
        commands = [{"seq": 0, "action": "prepare"}, {"seq": 1, "action": "arm", "args": {"round": 1}},
                    {"seq": 2, "action": "recover", "args": {"round": 1}},
                    {"seq": 3, "action": "release"}, {"seq": 4, "action": "close"}]
        real_write = os.pwrite
        observed = []

        def write(fd, data, offset):
            if self.actor.current_action == "recover" and fd == self.actor.fds.get("unlinked"):
                row = json.loads(self.actor.history.read_text().splitlines()[-1])
                self.assertEqual((row["event"], row["action"]), ("observation", "recover"))
                saved = row["unlinked_before_write"]
                self.assertEqual(os.fstat(fd).st_size, saved["size"])
                self.assertEqual(client.sha(os.pread(fd, saved["size"] + 1, 0)), saved["sha256"])
                self.assertEqual(offset, saved["size"], "post-proof mutation must append, not rewrite saved bytes")
                observed.append(row)
            return real_write(fd, data, offset)

        with patch.object(client.sys, "stdin", io.StringIO("\n".join(json.dumps(row) for row in commands) + "\n")), \
                patch.object(client.os, "pwrite", side_effect=write), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(self.actor.serve(), 0)
        self.assertTrue(observed)
        snapshot = client.load_snapshot(self.actor.history)
        client.compare(self.mount, snapshot)
        rows = [json.loads(line) for line in self.actor.history.read_text().splitlines()]
        broken = self.root / "missing-proof.jsonl"
        broken.write_text("\n".join(json.dumps(row) for row in rows if not
                                   (row["event"] == "observation" and row["action"] == "recover")) + "\n")
        with self.assertRaisesRegex(ValueError, "missing saved descriptor proof"):
            client.load_snapshot(broken)

    def test_nfs41_saved_profile_cannot_be_reinterpreted_as_native_xattr_evidence(self):
        oracle = client.sequential
        history = self.root / "nfs-history.jsonl"
        unsupported = OSError(errno.EOPNOTSUPP, "NFSv4.1 user xattrs unsupported")
        with patch.object(client.os, "setxattr", side_effect=unsupported), \
                patch.object(client.os, "getxattr", side_effect=unsupported), contextlib.redirect_stdout(io.StringIO()):
            oracle.record(self.mount, history, profile="nfs41")
            self.assertEqual(oracle.verify(self.mount, history, profile="nfs41"), 0)
            with self.assertRaisesRegex(AssertionError, "profile mismatch"):
                oracle.verify(self.mount, history)

    def test_completed_sequential_history_rejects_a_replaced_real_root_inode(self):
        oracle = client.sequential
        history = self.root / "completed-history.jsonl"
        oracle.record(self.mount, history)
        root = self.mount / json.loads(history.read_text().splitlines()[0])["name"]
        saved = self.mount / "saved-complete-root"
        root.rename(saved)
        shutil.copytree(saved, root, symlinks=True)
        with self.assertRaisesRegex(AssertionError, "state violates acknowledged"):
            oracle.verify(self.mount, history)


class ExternalEvidenceTests(unittest.TestCase):
    """Use real worker logs and the real cold CLI, not a substitute oracle."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir="/tmp/opencode")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.produced = 0
        self.mount, self.rows = self.produce_history("native", 2)

    def produce_history(self, profile, rounds, release=True):
        self.produced += 1
        directory = self.root / f"worker-{self.produced}"
        directory.mkdir()
        mount = directory / "mount"
        mount.mkdir()
        label = "kernel" if profile == "native" else "nfs-a"
        actor = client.Client(mount, directory / f"{label}.jsonl", label)
        self.addCleanup(actor.close)
        commands = [{"action": "prepare"}]
        for number in range(1, rounds + 1):
            commands += [{"action": action, "args": {"round": number}} for action in ("arm", "recover")]
        if release:
            commands.append({"action": "release"})
        commands.append({"action": "close"})
        for seq, request in enumerate(commands):
            request["seq"] = seq
        with contextlib.ExitStack() as stack:
            if profile == "nfs41":
                for syscall in ("setxattr", "getxattr", "listxattr"):
                    stack.enter_context(patch.object(client.os, syscall, side_effect=OSError(errno.EOPNOTSUPP, "NFSv4.1")))
            stack.enter_context(patch.object(client.sys, "stdin", io.StringIO("\n".join(map(json.dumps, commands)) + "\n")))
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            self.assertEqual(actor.serve(), 0)
        return mount, [json.loads(line) for line in actor.history.read_text().splitlines()]

    def cold_oracle(self, path, profile="native", mount=None):
        # Only the mount/isolation boundary is modeled. main(), load_snapshot()
        # and compare() run unchanged in an independent real Python process.
        bootstrap = r'''
import os,signal,subprocess,sys
from pathlib import Path
sys.path.insert(0,sys.argv.pop(1))
import g3_client
def no_signal(*args,**kwargs):
    raise AssertionError("real process signals forbidden")
os.kill=os.killpg=signal.pidfd_send_signal=no_signal
subprocess.Popen.send_signal=subprocess.Popen.terminate=subprocess.Popen.kill=no_signal
g3_client.exact_mount=lambda path: {"fstype":"offline-local-boundary"}
g3_client.external_path=lambda mount,path: Path(path)
sys.exit(g3_client.main())
'''
        return subprocess.run([sys.executable, "-c", bootstrap, str(ROOT / "scripts"), "verify", "--mount",
                               str(mount or self.mount), "--history", str(path), "--profile", profile],
                              capture_output=True, text=True, timeout=15)

    def save_rows(self, rows):
        path = self.root / "candidate.jsonl"
        path.write_text("\n".join(map(json.dumps, rows)) + "\n")
        return path

    def reject_rows(self, rows, profile="native", mount=None):
        path = self.save_rows(rows)
        with self.subTest(reader="loader"):
            with self.assertRaises((ValueError, KeyError, TypeError)):
                client.load_snapshot(path, profile)
        with self.subTest(reader="real-cold-CLI"):
            result = self.cold_oracle(path, profile, mount)
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            self.assertNotIn('{"verified": true}', result.stdout)

    def accept_rows(self, rows, profile="native", mount=None):
        path = self.save_rows(rows)
        snapshot = client.load_snapshot(path, profile)
        client.compare(mount or self.mount, snapshot, profile=profile)
        result = self.cold_oracle(path, profile, mount)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"verified": True})

    def ack(self, rows, action, occurrence=0):
        return [row["result"] for row in rows if row.get("event") == "ack" and row.get("action") == action][occurrence]

    def test_r6_empty_and_wrong_type_proofs_cannot_satisfy_external_evidence(self):
        for value in ({}, False, [], "proof", "", 0, 1, None,
                      {"inode": True, "size": -1, "sha256": "bad", "nlink": 1}):
            with self.subTest(value=value):
                rows = copy.deepcopy(self.rows)
                self.assertEqual(replace_descriptor_proofs(rows, lambda proof: value), 13)
                self.reject_rows(rows)

    def test_r6_every_proof_field_is_required_even_when_all_copies_match(self):
        for field in ("inode", "size", "sha256", "nlink"):
            with self.subTest(missing=field):
                rows = copy.deepcopy(self.rows)

                def missing(proof):
                    del proof[field]
                    return proof

                replace_descriptor_proofs(rows, missing)
                self.reject_rows(rows)

    def test_r6_proof_inode_and_size_have_strict_integer_types_and_ranges(self):
        for field, values in (("inode", (True, False, 0, -1, 1 << 64, 1.0, "1", [], {}, None)),
                              ("size", (True, False, -1, 1 << 63, 1.0, "1", [], {}, None))):
            for value in values:
                with self.subTest(field=field, value=value):
                    rows = copy.deepcopy(self.rows)
                    replace_descriptor_proofs(rows, lambda proof: {**proof, field: value})
                    self.reject_rows(rows)

    def test_r6_sha256_is_a_complete_canonical_digest_not_an_arbitrary_equal_value(self):
        for value in ("", "a" * 63, "a" * 65, "g" * 64, "A" * 64, "a" * 64 + "\n",
                      "０" * 64, 0, False, [], {}, None):
            with self.subTest(digest=value):
                rows = copy.deepcopy(self.rows)
                replace_descriptor_proofs(rows, lambda proof: {**proof, "sha256": value})
                self.reject_rows(rows)

    def test_r6_native_descriptor_link_count_is_exactly_integer_zero(self):
        for value in (False, True, -1, 1, 2, 1 << 64, 0.0, "0", None):
            with self.subTest(nlink=value):
                rows = copy.deepcopy(self.rows)
                replace_descriptor_proofs(rows, lambda proof: {**proof, "nlink": value})
                self.reject_rows(rows)

    def test_r6_nfs41_descriptor_link_count_is_exactly_integer_one(self):
        mount, original = self.produce_history("nfs41", 2)
        for value in (True, False, -1, 0, 2, 1 << 64, 1.0, "1", None):
            with self.subTest(nlink=value):
                rows = copy.deepcopy(original)
                replace_descriptor_proofs(rows, lambda proof: {**proof, "nlink": value})
                self.reject_rows(rows, "nfs41", mount)

    def test_r6_later_round_cannot_adopt_a_different_descriptor_inode(self):
        rows = copy.deepcopy(self.rows)
        self.ack(rows, "arm", 1)["unlinked_fsync_ack"]["inode"] += 1000000
        for row in rows:
            if row.get("seq") == 4:
                replace_descriptor_proofs([row], lambda proof: {**proof, "inode": proof["inode"] + 1000000})
                if row["event"] == "ack":
                    row["result"]["unlinked_inode"] += 1000000
        self.reject_rows(rows)

    def test_r6_flat_ack_fields_must_match_each_prepare_and_recover_proof(self):
        for action, occurrence in (("prepare", 0), ("recover", 0), ("recover", 1)):
            for field, value in (("unlinked_inode", 123), ("unlinked_size", 999),
                                 ("unlinked_data_sha256", "0" * 64), ("unlinked_nlink", 1),
                                 ("unlinked_inode", True), ("unlinked_size", False),
                                 ("unlinked_nlink", False)):
                with self.subTest(action=action, occurrence=occurrence, field=field, value=value):
                    rows = copy.deepcopy(self.rows)
                    self.ack(rows, action, occurrence)[field] = value
                    self.reject_rows(rows)
            for field in ("unlinked_inode", "unlinked_size", "unlinked_data_sha256", "unlinked_nlink"):
                with self.subTest(action=action, occurrence=occurrence, missing=field):
                    rows = copy.deepcopy(self.rows)
                    del self.ack(rows, action, occurrence)[field]
                    self.reject_rows(rows)

    def test_r6_consistently_changed_valid_fields_must_still_describe_the_workload_bytes(self):
        for data in (b"", b"not-the-acknowledged-unlinked-data"):
            with self.subTest(data=data):
                rows = copy.deepcopy(self.rows)
                replace_descriptor_proofs(rows, lambda proof: {**proof, "size": len(data), "sha256": client.sha(data)})
                for row in rows:
                    result = row.get("result", {})
                    if "unlinked_size" in result:
                        result["unlinked_size"] = len(data)
                        result["unlinked_data_sha256"] = client.sha(data)
                self.reject_rows(rows)

    def test_r6_arm_and_recover_ack_rounds_are_required_typed_and_ordered(self):
        for action, occurrence in (("arm", 0), ("recover", 0), ("arm", 1), ("recover", 1)):
            for value in (None, True, False, 0, -1, 3, 1 << 64, 1.0, "1"):
                with self.subTest(action=action, occurrence=occurrence, round=value):
                    rows = copy.deepcopy(self.rows)
                    self.ack(rows, action, occurrence)["round"] = value
                    self.reject_rows(rows)
            with self.subTest(action=action, occurrence=occurrence, missing="round"):
                rows = copy.deepcopy(self.rows)
                del self.ack(rows, action, occurrence)["round"]
                self.reject_rows(rows)
        for action, occurrence, value in (("arm", 0, 2), ("recover", 0, 2), ("arm", 1, 1), ("recover", 1, 1)):
            with self.subTest(action=action, occurrence=occurrence, repeated_or_skipped=value):
                rows = copy.deepcopy(self.rows)
                self.ack(rows, action, occurrence)["round"] = value
                self.reject_rows(rows)

    def test_r6_each_earlier_manifest_is_bound_to_the_original_prepare_root(self):
        for action, occurrence in (("prepare", 0), ("recover", 0)):
            for field, value in (("root", "g3-client-other"), ("profile", "nfs41"),
                                 ("ino", 123), ("kind", "file"), ("ino", True),
                                 ("ino", 0), ("ino", -1), ("ino", 1 << 64), ("ino", 1.0)):
                with self.subTest(action=action, field=field, value=value):
                    rows = copy.deepcopy(self.rows)
                    manifest = self.ack(rows, action, occurrence)["manifest"]
                    target = manifest if field in ("root", "profile") else manifest["entries"][""]
                    target[field] = value
                    self.reject_rows(rows)
            for value in ({}, False, [], "manifest", None):
                with self.subTest(action=action, malformed_manifest=value):
                    rows = copy.deepcopy(self.rows)
                    self.ack(rows, action, occurrence)["manifest"] = value
                    self.reject_rows(rows)
            with self.subTest(action=action, missing="root-entry"):
                rows = copy.deepcopy(self.rows)
                del self.ack(rows, action, occurrence)["manifest"]["entries"][""]
                self.reject_rows(rows)

    def test_r6_prepare_and_recover_manifests_are_required_not_optional(self):
        for action, occurrence in (("prepare", 0), ("recover", 0)):
            with self.subTest(action=action, occurrence=occurrence):
                rows = copy.deepcopy(self.rows)
                del self.ack(rows, action, occurrence)["manifest"]
                self.reject_rows(rows)

    def test_r6_unexpected_proofs_or_manifests_cannot_hide_in_other_action_acks(self):
        for action, field, value in (("arm", "unlinked_observation", {}), ("release", "unlinked_observation", {}),
                                     ("close", "unlinked_fsync_ack", {}), ("arm", "manifest", {}),
                                     ("release", "manifest", {})):
            with self.subTest(action=action, field=field):
                rows = copy.deepcopy(self.rows)
                self.ack(rows, action)[field] = value
                self.reject_rows(rows)

    def test_r6_records_cannot_hide_proofs_manifests_or_contradictory_rounds(self):
        for event, field, value in (("header", "unlinked_observation", {}), ("intent", "unlinked_fsync_ack", {}),
                                     ("ack", "unlinked_observation", {}), ("observation", "unlinked_fsync_ack", {}),
                                     ("intent", "manifest", {}), ("observation", "manifest", {}),
                                     ("header", "round", 7), ("intent", "round", 7),
                                     ("observation", "round", 7), ("ack", "round", 7)):
            with self.subTest(event=event, field=field):
                rows = copy.deepcopy(self.rows)
                next(row for row in rows if row["event"] == event)[field] = value
                self.reject_rows(rows)

    def test_r6_actions_obey_prepare_arm_recover_release_and_terminal_close_states(self):
        # Rechain only before-write copies to isolate the action-state defect.
        # The final real manifest and all ACK results stay otherwise unchanged.
        blocks = [[row for row in self.rows[1:] if row["seq"] == seq] for seq in range(7)]
        for name, order in (("prepare-twice", (0, 0, 1, 2, 3, 4, 5, 6)),
                            ("arm-twice", (0, 1, 1, 2, 3, 4, 5, 6)),
                            ("recover-twice", (0, 1, 2, 2, 3, 4, 5, 6)),
                            ("recover-without-arm", (0, 2, 3, 4, 5, 6)),
                            ("release-before-prepare", (5, 0, 1, 2, 3, 4, 6)),
                            ("release-during-dirty-round", (0, 1, 5, 2, 3, 4, 6)),
                            ("arm-after-release", (0, 5, 1, 2, 3, 4, 6)),
                            ("release-twice", (0, 1, 2, 3, 4, 5, 5, 6)),
                            ("close-before-prepare", (6, 0, 1, 2, 3, 4, 5)),
                            ("recover-after-close", (0, 1, 6, 2, 3, 4, 5)),
                            ("release-after-close", (0, 1, 2, 3, 4, 5, 6, 5)),
                            ("close-twice", (0, 1, 2, 3, 4, 5, 6, 6))):
            with self.subTest(order=name):
                rows, saved = [copy.deepcopy(self.rows[0])], None
                for seq, index in enumerate(order):
                    for row in copy.deepcopy(blocks[index]):
                        row["seq"] = seq
                        if row["event"] == "observation":
                            row["unlinked_before_write"] = copy.deepcopy(saved)
                        if row["event"] == "ack":
                            result = row["result"]
                            if "unlinked_before_write" in result:
                                result["unlinked_before_write"] = copy.deepcopy(saved)
                            saved = result.get("unlinked_observation", result.get("unlinked_fsync_ack", saved))
                        rows.append(row)
                self.reject_rows(rows)

    def test_r6_release_and_close_ack_values_are_checked(self):
        for action, field, value in (("release", "released_ns", False), ("release", "released_ns", -1),
                                     ("release", "released_ns", "1"), ("close", "closed", False),
                                     ("close", "closed", 1)):
            with self.subTest(action=action, field=field, value=value):
                rows = copy.deepcopy(self.rows)
                self.ack(rows, action)[field] = value
                self.reject_rows(rows)

    def test_r6_sequence_numbers_and_header_version_do_not_accept_numeric_lookalikes(self):
        for event, value in (("intent", False), ("ack", False), ("observation", True), ("ack", 0.0)):
            with self.subTest(event=event, value=value):
                rows = copy.deepcopy(self.rows)
                next(row for row in rows if row["event"] == event)["seq"] = value
                self.reject_rows(rows)
        rows = copy.deepcopy(self.rows)
        rows[0]["version"] = 2.0
        self.reject_rows(rows)

    def test_r6_valid_native_and_nfs41_prepared_recovered_released_closed_prefixes(self):
        for profile in ("native", "nfs41"):
            for rounds in (0, 1, 2):
                mount, rows = self.produce_history(profile, rounds)
                for terminal in ("ready", "released", "closed"):
                    with self.subTest(profile=profile, rounds=rounds, terminal=terminal):
                        prefix = rows if terminal == "closed" else [row for row in rows if
                                 row.get("action") not in (("close",) if terminal == "released" else ("release", "close"))]
                        self.accept_rows(prefix, profile, mount)
            mount, rows = self.produce_history(profile, 1, release=False)
            self.accept_rows(rows, profile, mount)

    def test_r6_missing_observation_and_unfinished_dirty_prefixes_still_fail(self):
        rows = [row for row in self.rows if not (row.get("event") == "observation" and row.get("seq") == 4)]
        self.reject_rows(rows)
        for last in (0, 1, 3):
            with self.subTest(incomplete_action=last):
                rows = [row for row in self.rows if row.get("seq", -1) <= last]
                if last == 0:
                    rows.pop()
                self.reject_rows(rows)


class ProcessBoundaryTests(unittest.TestCase):
    def test_abnormal_exit_and_closed_channel_fail_instead_of_ack(self):
        child = Mock(pid=99, returncode=7)
        child.poll.return_value = 7
        actor = gate.ClientProcess(child)
        with self.assertRaises(ValueError):
            actor.send("prepare")
        actor.pending = True
        with patch.object(gate.select, "select", return_value=([], [], [])), self.assertRaisesRegex(ValueError, "exited"):
            actor.receive(1)
        child.poll.return_value = None
        with patch.object(gate.select, "select", return_value=([child.stdout], [], [])), \
                patch.object(gate.os, "read", return_value=b""), self.assertRaisesRegex(ValueError, "closed"):
            actor.receive(1)

    def test_response_sequence_and_bounded_timeout(self):
        child = Mock(pid=99)
        child.poll.return_value = None
        actor = gate.ClientProcess(child)
        actor.send("prepare")
        with patch.object(gate.time, "monotonic", side_effect=[0, 2]), self.assertRaises(TimeoutError):
            actor.receive(1)
        actor.buffer = b'{"seq":999,"result":{}}\n'
        with self.assertRaisesRegex(ValueError, "unexpected client"):
            actor.receive(1)

    def test_cancel_propagates_without_becoming_a_successful_response(self):
        child = Mock(pid=99)
        child.poll.return_value = None
        actor = gate.ClientProcess(child)
        actor.send("recover", round=1)
        with patch.object(gate.select, "select", side_effect=base.Cancelled("test cancel")), \
                self.assertRaises(base.Cancelled):
            actor.receive(1)


class ControllerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir="/tmp/opencode")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.mount = self.root / "mount"
        self.mount.mkdir()
        self.conf = self.root / "ceph.conf"
        self.conf.write_text("[global]\n")
        self.trial = gate.ApplicationTrial(9000, self.mount, self.root / "artifacts", self.conf, [], True,
                                          binary=self.root / "ceph-rmds", keyring=self.root / "keyring",
                                          rust_filesystem=True, cold_mount=self.root / "cold",
                                          fuse_mount=self.root / "fuse", expected_fscid=44)
        self.trial.identity, self.trial.active = IDENTITY, dict(ACTIVE)
        self.trial.snapshot = {"id": 1, "boot_id": "boot", "unique_id": 100}
        self.trial.pidfd = 77
        self.trial.process_start = "123"

    def patch(self, obj, name, **kwargs):
        item = patch.object(obj, name, **kwargs)
        result = item.start()
        self.addCleanup(item.stop)
        return result

    def test_budget_refuses_before_workload_and_never_deletes(self):
        objects = [{"namespace": "", "name": f"rmds.checkpoint.0.1.{i}.manifest"} for i in range(40)]
        commands = []

        def read(args):
            commands.append(args)
            if "lspools" in args:
                return json.dumps([{"poolnum": 88, "poolname": "meta"}])
            if "ls" in args:
                return json.dumps(objects)
            return "165"

        self.patch(gate, "command", side_effect=read)
        with self.assertRaisesRegex(base.Prerequisite, "budget insufficient"):
            self.trial.capacity_check(2)
        self.assertFalse(self.trial.out.exists())
        self.assertFalse(any(token in args for args in commands for token in ("rm", "newfs", "reset", "setxattr")))

    def review_capacity_reader(self, count=15):
        objects = [{"namespace": "", "name": f"rmds.checkpoint.0.1.{i}.manifest"} for i in range(count)]

        def read(args, **kwargs):
            if "lspools" in args:
                return json.dumps([{"poolnum": 88, "poolname": "meta"}])
            if "ls" in args:
                return json.dumps(objects)
            return "165"

        return self.patch(gate, "command", side_effect=read)

    def assert_review_limit_20_refuses(self, text):
        self.conf.write_text(text)
        self.review_capacity_reader()
        with self.assertRaisesRegex(base.Prerequisite, "configured_limit.*20") as error:
            self.trial.capacity_check(2)
        self.assertEqual(base.failure_code(error.exception, self.trial), 2)
        self.assertFalse(self.trial.out.exists())

    def test_review_capacity_accepts_daemon_hyphen_spelling_not_default(self):
        self.assert_review_limit_20_refuses("[global]\nrmds-max-checkpoint-generations = 20\n")

    def test_review_capacity_accepts_daemon_uppercase_key_not_default(self):
        self.assert_review_limit_20_refuses("[global]\nRMDS_MAX_CHECKPOINT_GENERATIONS = 20\n")

    def test_review_capacity_accepts_daemon_uppercase_section_not_default(self):
        self.assert_review_limit_20_refuses("[MDS.RUST]\nrmds_max_checkpoint_generations = 20\n")

    def test_review_capacity_quoted_number_comments_and_normalized_overrides(self):
        self.assert_review_limit_20_refuses('[GLOBAL]\nrmds max checkpoint generations = 100 # fallback\n'
                                           '[MDS]\nRMDS-MAX-CHECKPOINT-GENERATIONS = 80\n'
                                           '[MDS.RUST]\nrmds_max_checkpoint_generations = "20" ; selected\n')

    def test_review_capacity_selected_standby_limit_is_checked_before_admission(self):
        self.conf.write_text("[global]\nrmds_max_checkpoint_generations=64\n"
                             "[MDS.FOLLOWER]\nRMDS-MAX-CHECKPOINT-GENERATIONS=20\n")
        self.review_capacity_reader()
        self.trial.takeover_pid = 9002
        self.patch(base.Trial, "preflight")
        self.patch(gate.shutil, "which", return_value="/mocked-required-tool")
        self.patch(gate, "process_identity", return_value={"argv": ["ceph-rmds", "-i", "follower", "--keyring", str(self.trial.keyring)]})
        with self.assertRaisesRegex(base.Prerequisite, "configured_limit.*20"):
            self.trial.preflight()
        self.assertFalse(self.trial.workload_started)

    def test_review_capacity_runtime_recheck_is_failure_not_application_pass(self):
        self.conf.write_text("[MDS.RUST]\nrmds-max-checkpoint-generations=20\n")
        self.review_capacity_reader()
        self.trial.workload_started = True
        with self.assertRaises(base.Prerequisite) as error:
            self.trial.capacity_check(1)
        self.assertEqual(base.failure_code(error.exception, self.trial), 1)
        self.assertFalse((self.trial.out / "result.json").exists())

    def test_review_capacity_budget_uses_configured_checkpoint_interval_seconds(self):
        self.conf.write_text("[MDS.RUST]\nRMDS-CHECKPOINT-INTERVAL=1\n")
        self.review_capacity_reader()
        with self.assertRaisesRegex(base.Prerequisite, "budget insufficient"):
            self.trial.capacity_check(2)

    def test_review_capacity_budget_uses_configured_record_threshold(self):
        self.conf.write_text("[mds.rust]\nrmds checkpoint records=1\n")
        self.review_capacity_reader()
        with self.assertRaisesRegex(base.Prerequisite, "budget insufficient"):
            self.trial.capacity_check(2)

    def test_capacity_valid_spellings_quotes_comments_and_u64_values_are_not_defaults(self):
        read = self.review_capacity_reader()
        for key in ("rmds_max_checkpoint_generations", "rmds-max-checkpoint-generations",
                    "rmds max checkpoint generations", "RMDS_MAX_CHECKPOINT_GENERATIONS",
                    "RMDS-MAX checkpoint_generations"):
            for section in ("GLOBAL", "MDS", "MDS.RUST"):
                for number in ("96", "+096", '"96"'):
                    with self.subTest(key=key, section=section, value=number):
                        self.conf.write_text(f"[{section}]\n{key}={number} ; comment\n")
                        result = self.trial.capacity_check(2)
                        self.assertEqual(result["configured_limit"], 96)
                        self.assertEqual(result["planned_additional_generation_budget"], 36)
        self.conf.write_text('[global]\nrmds_max_checkpoint_generations=+18446744073709551615\n')
        self.assertEqual(self.trial.capacity_check(2)["configured_limit"], (1 << 64) - 1)
        self.conf.write_text('[global]\nrmds_max_checkpoint_generations=+' + '0' * 5000 + '96\n')
        self.assertEqual(self.trial.capacity_check(2)["configured_limit"], 96)
        self.assertGreater(read.call_count, 0)

    def test_capacity_section_precedence_last_normalized_key_and_threshold_units(self):
        self.review_capacity_reader()
        self.conf.write_text('[GLOBAL]\nrmds-max-checkpoint-generations=90\n'
                             '[MDS]\nRMDS_MAX_CHECKPOINT_GENERATIONS=96\n'
                             '[MDS.RUST]\nrmds max checkpoint generations=100\n'
                             'RMDS-MAX-CHECKPOINT-GENERATIONS="120" # last normalized alias\n'
                             'rmds-checkpoint-interval="60"\nrmds checkpoint records=32\n')
        result = self.trial.capacity_check(2)
        self.assertEqual(result["configured_limit"], 120)
        self.assertEqual(result["checkpoint_interval_seconds"], 60)
        self.assertEqual(result["checkpoint_record_threshold"], 32)
        self.assertEqual(result["planned_additional_generation_budget"], 20)

    def test_capacity_rejects_daemon_invalid_suffix_units_signs_and_overflow(self):
        read = self.review_capacity_reader()
        for key, value in (("rmds_max_checkpoint_generations", "20KiB"),
                           ("rmds_max_checkpoint_generations", "20 generations"),
                           ("rmds_max_checkpoint_generations", "-20"),
                           ("rmds_max_checkpoint_generations", "1_000"),
                           ("rmds_max_checkpoint_generations", "١٢٠"),
                           ("rmds_max_checkpoint_generations", str(1 << 64)),
                           ("rmds_max_checkpoint_generations", '" 120 "'),
                           ("rmds_checkpoint_interval", "30s"), ("rmds_checkpoint_interval", "0"),
                           ("rmds_checkpoint_records", "16records"), ("rmds_checkpoint_records", "0")):
            with self.subTest(key=key, value=value):
                self.conf.write_text(f"[global]\n{key}={value}\n")
                read.reset_mock()
                with self.assertRaises(base.Prerequisite):
                    self.trial.capacity_check(2)
                read.assert_not_called()

    def test_capacity_does_not_invent_configuration_lines_from_python_separators(self):
        self.review_capacity_reader()
        for separator in (b"\x1c", b"\r", "\u2028".encode()):
            with self.subTest(separator=separator):
                self.conf.write_bytes(b"[global]\nrmds_max_checkpoint_generations=20" + separator +
                                      b"rmds_max_checkpoint_generations=96\n")
                with self.assertRaises(base.Prerequisite):
                    self.trial.capacity_check(2)

    def test_missing_required_tool_is_prerequisite_before_any_native_probe(self):
        self.patch(gate.shutil, "which", return_value=None)
        probe = self.patch(base, "cluster_state")
        send = self.patch(gate.signal, "pidfd_send_signal")
        with self.assertRaisesRegex(base.Prerequisite, "missing required tools"):
            self.trial.preflight()
        probe.assert_not_called()
        send.assert_not_called()
        self.assertFalse(self.trial.out.exists())

    def test_mds_stop_deadline_never_launches_replacement(self):
        self.trial.round_dir = self.root
        self.patch(self.trial, "check_target")
        self.patch(self.trial, "assert_clients")
        self.patch(self.trial, "assert_locks_held")
        send = self.patch(gate.signal, "pidfd_send_signal")
        self.patch(gate.select, "poll").return_value.poll.return_value = []
        restart = self.patch(self.trial, "start_daemon")
        with self.assertRaises(TimeoutError):
            self.trial.signal_mds()
        send.assert_called_once_with(77, signal.SIGKILL)
        restart.assert_not_called()

    def test_combined_failure_signals_both_owned_targets_before_waiting_for_exit(self):
        self.trial.mode = "mds-ganesha"
        self.trial.round_dir = self.root
        self.trial.ganesha = Mock()
        order = []
        self.patch(self.trial, "check_target")
        self.patch(self.trial, "assert_clients")
        self.patch(self.trial, "assert_locks_held")
        self.patch(gate.signal, "pidfd_send_signal", side_effect=lambda fd, sig: order.append("mds-signal"))

        def signal_helper(wait=True):
            self.assertFalse(wait)
            order.append("ganesha-signal")
            return time.monotonic_ns()

        self.patch(self.trial, "signal_ganesha", side_effect=signal_helper)
        poll = self.patch(gate.select, "poll")
        poll.return_value.poll.side_effect = lambda timeout: order.append("wait-mds") or [(77, 1)]
        self.trial.ganesha.wait.side_effect = lambda **kwargs: order.append("wait-ganesha")
        self.trial.signal_mds()
        self.assertEqual(order, ["mds-signal", "ganesha-signal", "wait-mds", "wait-ganesha"])

    def test_failed_or_missing_selected_assertion_cannot_write_result(self):
        self.patch(self.trial, "start_fuse")
        self.patch(self.trial, "start_contenders")
        self.patch(self.trial, "capacity_check", return_value={})
        self.patch(self.trial, "assert_clients")
        self.patch(self.trial, "assert_locks_held")
        self.patch(self.trial, "check_target")
        self.patch(self.trial, "start_sequential")
        self.patch(self.trial, "signal_mds", return_value=1)
        self.patch(self.trial, "start_daemon", return_value=9001)
        self.patch(self.trial, "wait_active", return_value=(2, IDENTITY, {**ACTIVE, "gid": 102}))
        self.patch(self.trial, "adopt_active")

        def prepare(label, mount):
            actor = Mock()
            actor.request.return_value = {"durability": "write-return-only", "round": 1}
            actor.receive.return_value = {"round": 1, "first_success_monotonic_ns": 3,
                                          "assertions": {case: case != "positive_symlinks" for case in client.CASES}}
            self.trial.clients[label] = actor
            return {}

        self.patch(self.trial, "start_client", side_effect=prepare)
        verify = self.patch(self.trial, "cold_compare")
        with self.assertRaisesRegex(ValueError, "assertion missing/false"):
            self.trial.execute()
        verify.assert_not_called()
        self.assertFalse((self.trial.out / "result.json").exists())

    def test_cleanup_stops_descriptor_holders_before_any_unmount_and_keeps_busy_mds(self):
        order = []
        worker, contender, sequential, daemon = [Mock(pid=i) for i in (10, 11, 12, 13)]
        self.trial.clients = {"kernel": Mock(child=worker)}
        self.trial.contenders = [("kernel", "fcntl", contender, self.root / "locks")]
        self.trial.sequential = [(sequential, self.root / "seq")]
        self.trial.children = [daemon]
        self.trial.workload_started = True
        self.patch(self.trial, "stop_owned_child", side_effect=lambda child: order.append(child.pid))
        self.patch(self.trial, "check_mount")
        self.patch(base, "exact_mount", return_value={"id": 1})

        def busy(args, **kwargs):
            order.append("umount")
            raise TimeoutError("busy test client")

        self.patch(base, "command", side_effect=busy)
        with contextlib.redirect_stderr(io.StringIO()):
            self.trial.cleanup()
        self.assertEqual(order, [10, 11, 12, "umount"])
        self.assertIsNotNone(self.trial.snapshot)

    def test_run_cancellation_invokes_owned_cleanup_under_lock(self):
        self.patch(self.trial, "preflight")
        self.patch(self.trial, "check_mount")
        self.patch(self.trial, "check_target")
        self.patch(self.trial, "execute", side_effect=base.Cancelled("cancel during worker I/O"))
        cleanup = self.patch(self.trial, "cleanup")
        lock = self.root / "local-lock"
        item = patch.object(base, "open", side_effect=lambda path, mode: lock.open(mode), create=True)
        item.start()
        self.addCleanup(item.stop)
        self.patch(base.fcntl, "flock")
        with self.assertRaises(base.Cancelled):
            self.trial.run()
        cleanup.assert_called_once_with()
        self.assertFalse((self.trial.out / "result.json").exists())

    def test_cold_comparison_error_does_not_unmount_a_replacement_or_write_result(self):
        self.trial.round_dir = self.root
        self.patch(self.trial, "assert_clients")
        self.patch(self.trial, "mount_kernel", return_value={"boot_id": "boot", "unique_id": 200, "id": 2})
        self.patch(self.trial, "verify_at", side_effect=OSError(errno.EIO, "cold comparison failed"))
        command = self.patch(gate, "command")
        with self.assertRaises(OSError):
            self.trial.cold_compare(("kernel", "fuse"))
        command.assert_not_called()
        self.assertIn(self.trial.cold_mount, self.trial.extra_mounts)
        self.assertFalse((self.trial.out / "result.json").exists())

    def test_same_unique_identity_cannot_claim_independent_cold_client(self):
        self.trial.round_dir = self.root
        self.patch(self.trial, "assert_clients")
        self.patch(self.trial, "mount_kernel", return_value=dict(self.trial.snapshot))
        verify = self.patch(self.trial, "verify_at")
        with self.assertRaisesRegex(ValueError, "previous unique identity"):
            self.trial.cold_compare(("kernel", "fuse"))
        verify.assert_not_called()

    def test_replaced_cold_mount_is_retained_instead_of_unmounted(self):
        self.trial.round_dir = self.root
        self.patch(self.trial, "assert_clients")
        self.patch(self.trial, "mount_kernel", return_value={"boot_id": "boot", "unique_id": 200, "id": 2})
        self.patch(self.trial, "verify_at")
        self.patch(gate, "require_same_mount", side_effect=ValueError("unique mount replaced"))
        command = self.patch(gate, "command")
        with self.assertRaisesRegex(ValueError, "unique mount replaced"):
            self.trial.cold_compare(("kernel", "fuse"))
        command.assert_not_called()
        self.assertIn(self.trial.cold_mount, self.trial.extra_mounts)

    def test_bounded_ganesha_recovery_evidence_is_not_protocol_qualification(self):
        data = self.root / "data"
        (data / "ganesha-recovery/recov/client").mkdir(parents=True)
        (data / "ganesha-recovery/recov/client/state").write_bytes(b"retained-state")
        (data / "ganesha-recovery/ganesha.conf").write_text("stable recovery config")
        (data / "log").mkdir()
        (data / "log/ganesha-recovery.log").write_text("grace start observation\n")
        self.trial.round_dir = self.root
        with patch.dict(os.environ, {"DEVCLUSTER_DATA": str(data)}):
            self.trial.ganesha_stable = self.trial.ganesha_fingerprint()
            self.trial.capture_ganesha_evidence("first")
            result = json.loads((self.root / "ganesha-first.json").read_text())
            self.assertFalse(result["RESET_shared_session_and_NFS_reclaim_proven"])
            self.assertEqual(len(result["recovery_entries"]), 2)
            (data / "log/ganesha-recovery.log").write_text("")
            with self.assertRaisesRegex(ValueError, "rotated/truncated"):
                self.trial.capture_ganesha_evidence("truncated")

    def test_unvalidated_nfs_mount_retains_service_children(self):
        self.trial.nfs_mount = self.root / "nfs"
        self.trial.children = [Mock(pid=20)]
        self.patch(gate, "exact_physical_mount", return_value={"id": 123})
        stop = self.patch(self.trial, "stop_owned_child")
        with contextlib.redirect_stderr(io.StringIO()):
            self.trial.cleanup()
        stop.assert_not_called()

    def test_ganesha_only_controller_never_signals_mds_and_stays_partial(self):
        trial = self.trial
        trial.mode, trial.rounds = "ganesha-only", 1
        trial.fuse_mount, trial.nfs_mount = None, self.root / "nfs"
        trial.profile = "nfs41"
        trial.cases = client.selected_cases(trial.profile)
        for method in ("start_ganesha", "capture_ganesha_evidence", "start_contenders", "assert_clients",
                       "assert_locks_held", "check_target", "start_sequential", "verify_at", "cold_compare", "finish_locks"):
            self.patch(trial, method)
        self.patch(trial, "capacity_check", return_value={})
        self.patch(trial, "mount_nfs", return_value={"id": 3})
        helper_signal = self.patch(trial, "signal_ganesha", return_value=1)
        mds_signal = self.patch(trial, "signal_mds")
        restart = self.patch(trial, "start_daemon")

        def start(label, mount):
            actor = Mock()
            actor.request.return_value = {"round": 1, "durability": "write-return-only"}
            actor.receive.return_value = {"round": 1, "first_success_monotonic_ns": 2,
                                         "assertions": {case: True for case in trial.cases}}
            trial.clients[label] = actor
            return {}

        self.patch(trial, "start_client", side_effect=start)
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(trial.execute(), 77)
        helper_signal.assert_called_once_with()
        mds_signal.assert_not_called()
        restart.assert_not_called()
        result = json.loads((trial.out / "result.json").read_text())
        self.assertEqual(result["result"], "PARTIAL")
        self.assertFalse(result["full_g3_qualified"])
        self.assertNotIn("xattrs", result["selected_cases"])
        self.assertIn("xattrs", result["excluded_cases"])

    def test_nfs_mount_requires_and_reports_the_selected_v41_protocol(self):
        mount = self.root / "nfs"
        self.patch(gate, "require_unmounted")
        command = self.patch(gate, "command", return_value="")
        identity = {"id": 3, "boot_id": "boot", "unique_id": 300}
        self.patch(gate, "capture_mount_identity", return_value=identity)
        row = {"fstype": "nfs4", "source": "127.0.0.1:/a", "options": "port=12049,vers=4.1"}
        probe = self.patch(gate, "exact_physical_mount", return_value=row)
        self.assertEqual(self.trial.mount_nfs(mount), identity)
        self.assertIn("vers=4.1", command.call_args.args[0][4])
        probe.return_value = dict(row, options="port=12049,vers=4.2")
        with self.assertRaisesRegex(ValueError, "identity/port"):
            self.trial.mount_nfs(mount)

    def test_named_wire_loss_gate_is_missing_prerequisite_not_random_kill_success(self):
        argv = ["reply-loss", "0", str(self.mount), str(self.root / "no-artifacts"), "--expected-fscid", "44",
                "--binary", "/not-executed/ceph-rmds", "--keyring", "/not-read/keyring", "--cold-mount", str(self.root / "cold")]
        result = subprocess.run([sys.executable, str(ROOT / "scripts/g3_trial.py"), *argv], capture_output=True,
                                text=True, timeout=10, env={**os.environ, "RMDS_RECOVERY_READY": "yes", "RMDS_G3_READY": "yes"})
        self.assertEqual(result.returncode, 2)
        self.assertIn("fault seam is not implemented", result.stderr)
        self.assertFalse((self.root / "no-artifacts").exists())

    def test_ganesha_stable_identity_change_refuses_signal(self):
        self.trial.ganesha = Mock(pid=20)
        self.trial.ganesha.poll.return_value = None
        self.trial.ganesha_identity = {"start": "123"}
        self.trial.ganesha_stable = {"config_sha256": "original"}
        self.patch(gate.os, "pidfd_open", return_value=77)
        self.patch(gate.os, "close")
        self.patch(gate, "process_identity", return_value={"start": "123", "exe": "ganesha.nfsd",
                                                          "listeners": {"127.0.0.1:12049"}})
        self.patch(self.trial, "ganesha_fingerprint", return_value={"config_sha256": "changed"})
        send = self.patch(gate.signal, "pidfd_send_signal")
        with self.assertRaisesRegex(ValueError, "stable identity changed"):
            self.trial.signal_ganesha()
        send.assert_not_called()

    def test_test_server_on_system_port_is_never_selected(self):
        self.trial.ganesha = Mock(pid=20)
        self.trial.ganesha.poll.return_value = None
        self.trial.ganesha_identity = {"start": "123"}
        self.patch(gate.os, "pidfd_open", return_value=77)
        self.patch(gate.os, "close")
        self.patch(gate, "process_identity", return_value={"start": "123", "exe": "ganesha.nfsd",
                                                          "listeners": {"127.0.0.1:2049"}})
        send = self.patch(gate.signal, "pidfd_send_signal")
        with self.assertRaises(ValueError):
            self.trial.signal_ganesha()
        send.assert_not_called()

    def run_local_controller(self, rounds, fault=None, expect_failure=False, cold_fault=None, mode="native"):
        """Execute the real controller, clients, contenders and comparison CLIs.

        Only mount/daemon/native-capacity boundaries are injected. Child CLIs
        use local directories through explicit mount/isolation probes. There
        are no fake executables, live mounts, daemon starts, or real signals.
        """
        if mode != "native":
            self.trial = gate.ApplicationTrial(9000, self.mount, self.root / "artifacts", self.conf, [], True,
                                               mode=mode, nfs_mount=self.mount, binary=self.root / "ceph-rmds",
                                               keyring=self.root / "keyring", rust_filesystem=True,
                                               cold_mount=self.root / "cold-view", expected_fscid=44)
            self.trial.identity, self.trial.active = IDENTITY, dict(ACTIVE)
            self.trial.snapshot = {"id": 1, "boot_id": "boot", "unique_id": 100}
            self.trial.process_start = "123"
            self.trial.cold_mount.symlink_to(self.mount, target_is_directory=True)
        trial = self.trial
        trial.rounds = rounds
        trial.binary.write_bytes(b"reviewed binary boundary, never executed")
        trial.binary_hash = hashlib.sha256(trial.binary.read_bytes()).hexdigest()
        trial.conf_hash = hashlib.sha256(self.conf.read_bytes()).hexdigest()
        # Three view labels refer to the same local filesystem, not pretend
        # independent clients. Native independence is tested at the injected
        # mount boundary and still requires a later live qualification.
        if mode == "native":
            trial.fuse_mount = trial.mount
            trial.cold_mount = trial.mount
        real_popen, real_read = subprocess.Popen, Path.read_bytes
        children, signals = [], []
        bootstrap = r'''
import importlib.util,sys
from pathlib import Path
script=sys.argv[1]
sys.path.insert(0,str(Path(script).parent))
spec=importlib.util.spec_from_file_location("offline_g3_actor",script)
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
module.exact_mount=lambda path: {"fstype":"offline-local-boundary"}
module.external_path=lambda mount,path: Path(path)
if "--profile" in sys.argv and sys.argv[sys.argv.index("--profile")+1]=="nfs41":
    import errno
    def unsupported(*args,**kwargs):
        raise OSError(errno.EOPNOTSUPP,"modeled NFSv4.1 user xattrs unsupported")
    module.os.setxattr=unsupported
    module.os.getxattr=unsupported
    module.os.listxattr=unsupported
sys.argv=sys.argv[1:]
sys.exit(module.main())
'''

        def launch(args, **kwargs):
            self.assertEqual(args[0], sys.executable)
            self.assertIn(Path(args[1]).name, ("g3_client.py", "crash-history.py", "restart_history.py"))
            child = real_popen([sys.executable, "-c", bootstrap, *args[1:]], **kwargs)
            children.append(child)
            return child

        def native_command(args, **kwargs):
            if args[0] == "umount":
                return ""
            return base.command(args, **kwargs)

        def read(path):
            return real_read(trial.binary) if str(path).startswith("/proc/") and path.name == "exe" else real_read(path)

        process = {"exe": "ceph-rmds", "argv": [str(trial.binary), "-i", "rust", "-c", str(self.conf)],
                   "start": "123", "listeners": {"127.0.0.1:7401"}}

        def signal_boundary(fd, sig):
            signals.append(sig)
            if fault:
                fault(trial, len(signals))

        def cold_boundary(path):
            if cold_fault:
                cold_fault(trial, len(trial.round_results) + 1)
            return {"id": 2, "fstype": "ceph", "boot_id": "boot", "unique_id": 200 + len(trial.round_results)}

        def nfs_boundary(path):
            return {"id": 3, "fstype": "nfs4", "boot_id": "boot",
                    "unique_id": 500 if path == trial.nfs_mount else 600 + len(trial.round_results)}

        def helper_signal(wait=True):
            signal_boundary(0, signal.SIGKILL)
            return time.monotonic_ns()

        trial.pidfd = os.open("/dev/null", os.O_RDONLY)
        error, code = None, None
        with contextlib.ExitStack() as stack:
            boundaries = [patch.object(trial, "start_fuse"), patch.object(trial, "assert_clients"),
                          patch.object(trial, "check_target"), patch.object(trial, "capacity_check", return_value={}),
                          patch.object(trial, "start_daemon", return_value=9001),
                          patch.object(trial, "wait_active", side_effect=lambda pid: (time.monotonic_ns(), IDENTITY,
                                                                                  {**ACTIVE, "gid": 102 + len(trial.round_results)})),
                          patch.object(trial, "mount_kernel", side_effect=cold_boundary),
                          patch.object(trial, "mount_nfs", side_effect=nfs_boundary),
                          patch.object(trial, "start_ganesha"), patch.object(trial, "capture_ganesha_evidence"),
                          patch.object(trial, "signal_ganesha", side_effect=helper_signal),
                          patch.object(gate, "require_same_mount"), patch.object(gate, "require_unmounted"),
                          patch.object(gate, "process_identity", return_value=process),
                          patch.object(gate.os, "pidfd_open", side_effect=lambda pid: os.open("/dev/null", os.O_RDONLY)),
                          patch.object(gate.Path, "read_bytes", read),
                          patch.object(gate.signal, "pidfd_send_signal", side_effect=signal_boundary),
                          patch.object(gate.subprocess, "Popen", side_effect=launch),
                          patch.object(gate, "command", side_effect=native_command),
                          contextlib.redirect_stdout(io.StringIO())]
            for boundary in boundaries:
                stack.enter_context(boundary)
            poll = stack.enter_context(patch.object(gate.select, "poll"))
            poll.return_value.poll.return_value = [(1, 1)]
            try:
                code = trial.execute()
            except Exception as caught:
                error = caught
            finally:
                # Even an assertion failure must not leave local lock waiters.
                # Use the real client close protocol, never process signals.
                for actor in trial.clients.values():
                    if actor.child.poll() is None:
                        if actor.pending:
                            try:
                                actor.receive(10)
                            except ValueError:
                                # A deliberately failed real client closes its
                                # pipe and exits. Reap it; do not send a signal.
                                actor.child.wait(timeout=10)
                        if actor.child.poll() is None:
                            actor.request("close", 10)
                        actor.child.wait(timeout=10)
                    for pipe in (actor.child.stdin, actor.child.stdout):
                        if pipe and not pipe.closed:
                            pipe.close()
                for child in children:
                    if child.poll() is None and child.stdin is not None and not child.stdin.closed:
                        child.stdin.write(b"continue\n")
                        child.stdin.close()
                    child.wait(timeout=10)
                os.close(trial.pidfd)
        if expect_failure:
            self.assertIsNotNone(error, "complete controller falsely passed corrupted acknowledged state")
            self.assertEqual(base.failure_code(error, trial), 1)
            self.assertFalse((trial.out / "result.json").exists())
            self.assertLessEqual(len(signals), rounds)
            return 1, error
        if error is not None:
            raise error
        self.assertEqual(signals, [signal.SIGKILL] * rounds)
        return code, json.loads((trial.out / "result.json").read_text())

    def test_real_complete_controller_two_rounds_requires_all_client_cold_and_lock_proofs(self):
        code, result = self.run_local_controller(2)
        self.assertEqual(code, 0)
        self.assertEqual(result["result"], "APPLICATION_PASS")
        self.assertFalse(result["full_g3_qualified"])
        self.assertEqual(len(result["rounds"]), 2)
        proof = json.loads((self.trial.out / "lock-proof.json").read_text())
        self.assertEqual({(row["client"], row["kind"]) for row in proof},
                         {(label, kind) for label in ("kernel", "fuse") for kind in ("fcntl", "flock")})
        for round in result["rounds"]:
            self.assertTrue(round["cold_comparison"])
            self.assertTrue(round["sequential_complete"])
            self.assertIsNone(round["protocol_reconnect_seconds"])
            for assertions in round["client_assertions"].values():
                self.assertTrue(all(assertions[case] for case in client.CASES))

    def test_single_round_is_partial_even_when_real_application_assertions_pass(self):
        code, result = self.run_local_controller(1)
        self.assertEqual(code, 77)
        self.assertEqual(result["result"], "PARTIAL")
        self.assertFalse(result["full_g3_qualified"])

    def test_review_complete_two_round_controller_rejects_old_history_corruption(self):
        def corrupt(trial, number):
            if number == 2:
                history = trial.out / "round-001/sequential-kernel.jsonl"
                root = trial.mount / json.loads(history.read_text().splitlines()[0])["name"]
                (root / "alias").write_bytes(b"round-one acknowledged data lost")

        _, error = self.run_local_controller(2, fault=corrupt, expect_failure=True)
        self.assertIsInstance(error, subprocess.CalledProcessError)
        self.assertIn(str(self.trial.out / "round-001/sequential-kernel.jsonl"), error.cmd)
        self.assertIn("state violates acknowledged", error.stderr)

    def test_review_complete_two_round_controller_rejects_old_namespace_deletion(self):
        def remove(trial, number):
            if number == 2:
                history = trial.out / "round-001/sequential-fuse.jsonl"
                root = trial.mount / json.loads(history.read_text().splitlines()[0])["name"]
                (root / "alias").unlink()
                root.rmdir()

        _, error = self.run_local_controller(2, fault=remove, expect_failure=True)
        self.assertIsInstance(error, subprocess.CalledProcessError)
        self.assertIn(str(self.trial.out / "round-001/sequential-fuse.jsonl"), error.cmd)
        self.assertIn("state violates acknowledged", error.stderr)

    def test_review_complete_controller_rejects_lost_unlinked_fd_bytes(self):
        def truncate(trial, number):
            if number != 1:
                return
            child = trial.clients["kernel"].child
            paths = [path for path in Path(f"/proc/{child.pid}/fd").iterdir()
                     if os.readlink(path).endswith("/unlinked (deleted)")]
            self.assertEqual(len(paths), 1)
            fd = os.open(paths[0], os.O_RDWR)
            try:
                os.ftruncate(fd, 0)
            finally:
                os.close(fd)

        self.run_local_controller(2, fault=truncate, expect_failure=True)
        self.assertIn("unlinked-open descriptor", (self.trial.out / "kernel-stderr.log").read_text())

    def test_later_cold_comparison_rechecks_earlier_sequential_history(self):
        def corrupt_after_mounted_checks(trial, number):
            if number == 2:
                history = trial.out / "round-001/sequential-kernel.jsonl"
                root = trial.mount / json.loads(history.read_text().splitlines()[0])["name"]
                (root / "alias").write_bytes(b"cold-only round-one loss")

        _, error = self.run_local_controller(2, cold_fault=corrupt_after_mounted_checks, expect_failure=True)
        self.assertIsInstance(error, subprocess.CalledProcessError)
        self.assertIn(str(self.trial.out / "round-001/sequential-kernel.jsonl"), error.cmd)
        self.assertIn("state violates acknowledged", error.stderr)

    def test_r6_two_round_controller_rejects_empty_proofs_at_later_cold_boundary(self):
        injected = []

        def corrupt_proofs(trial, number):
            if number != 2:
                return
            for label in ("kernel", "fuse"):
                path = trial.out / f"{label}.jsonl"
                rows = [json.loads(line) for line in path.read_text().splitlines()]
                count = replace_descriptor_proofs(rows, lambda proof: {})
                self.assertEqual(count, 13)
                path.write_text("\n".join(map(json.dumps, rows)) + "\n")
                injected.append(label)

        _, error = self.run_local_controller(2, cold_fault=corrupt_proofs, expect_failure=True)
        self.assertEqual(injected, ["kernel", "fuse"])
        self.assertIsInstance(error, subprocess.CalledProcessError)
        self.assertIn(str(self.trial.out / "kernel.jsonl"), error.cmd)
        self.assertIn("descriptor proof", error.stderr)
        self.assertEqual(self.trial.phase, "round2:comparison")

    def test_r6_two_round_controller_rejects_earlier_prepare_root_drift_at_later_cold_boundary(self):
        injected = []

        def corrupt_prepare(trial, number):
            if number != 2:
                return
            for label in ("kernel", "fuse"):
                path = trial.out / f"{label}.jsonl"
                rows = [json.loads(line) for line in path.read_text().splitlines()]
                prepare = next(row["result"] for row in rows if row.get("event") == "ack" and
                               row.get("action") == "prepare")
                prepare["manifest"]["entries"][""]["ino"] += 1000000
                path.write_text("\n".join(map(json.dumps, rows)) + "\n")
                injected.append(label)

        _, error = self.run_local_controller(2, cold_fault=corrupt_prepare, expect_failure=True)
        self.assertEqual(injected, ["kernel", "fuse"])
        self.assertIsInstance(error, subprocess.CalledProcessError)
        self.assertIn(str(self.trial.out / "kernel.jsonl"), error.cmd)
        self.assertIn("root identity", error.stderr)
        self.assertEqual(self.trial.phase, "round2:comparison")

    def test_full_ganesha_local_application_profile_never_calls_unsupported_nfs_xattrs(self):
        code, result = self.run_local_controller(2, mode="ganesha-only")
        self.assertEqual(code, 77)
        self.assertEqual(result["result"], "PARTIAL")
        self.assertEqual(result["profile"], "nfs41")
        self.assertNotIn("xattrs", result["selected_cases"])
        self.assertIn("xattrs", result["excluded_cases"])
        for round in result["rounds"]:
            for assertions in round["client_assertions"].values():
                self.assertIs(assertions["xattrs"], False)
                self.assertIs(assertions["nfs41_ack_history"], True)
        self.assertEqual(len(result["rounds"][-1]["verified_sequential_histories"]), 4)


if __name__ == "__main__":
    unittest.main()
