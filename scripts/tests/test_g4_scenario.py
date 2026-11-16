"""G4 client scenarios. The live mount runs only for the target-g4 daemon.

    python3 scripts/tests/test_g4_scenario.py
    sudo env G4_MOUNT=/mnt/rmds-fuse ... python3 scripts/tests/test_g4_scenario.py
"""
import errno
import os
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import g4_scenario as g4


class ScenarioTableTests(unittest.TestCase):
    def test_steps_cover_adoption_exdev_clear_and_quota(self):
        kinds = [step[0] for step in g4.STEPS]
        self.assertLess(kinds.index("setxattr"), kinds.index("rename"))
        self.assertIn(("rename", "volumes/a/inner/fa", "volumes/b/fa", errno.EXDEV), g4.STEPS)
        self.assertIn(("link", "volumes/b/l", "volumes/a/inner/fa", errno.EXDEV), g4.STEPS)
        self.assertIn(("rename", "volumes/a/inner/fa", "volumes/a/fa2", 0), g4.STEPS)
        self.assertIn(
            ("removexattr", "volumes/a", "ceph.dir.subvolume", errno.ENOTEMPTY), g4.STEPS)
        self.assertIn(("setxattr", "volumes/c", "ceph.dir.subvolume", "1", errno.EINVAL), g4.STEPS)
        self.assertIn(("setxattr", "volumes/q", "ceph.quota.max_files", "1", 0), g4.STEPS)
        self.assertIn(("create", "volumes/q/over", errno.EDQUOT), g4.STEPS)
        text = " ".join(str(part) for step in g4.STEPS for part in step)
        self.assertNotIn("ganesha", text.lower())

    def test_paths_stay_inside_the_scratch_directory(self):
        for step in g4.STEPS:
            relative = step[1]
            self.assertNotIn("..", Path(relative).parts)
            self.assertFalse(Path(relative).is_absolute())

    def test_a_target_g3_daemon_is_refused(self):
        with self.assertRaisesRegex(g4.WrongDaemon, "target-g3"):
            g4.require_g4_daemon([
                "/mnt/igor/github/salieri11/rust-mds/target-g3/debug/ceph-rmds -i rust",
            ])

    def test_a_target_g4_daemon_is_accepted(self):
        g4.require_g4_daemon([
            "/mnt/igor/github/salieri11/rust-mds/target-g4/debug/ceph-rmds -i rust",
        ])

    def test_no_daemon_is_refused(self):
        with self.assertRaisesRegex(g4.WrongDaemon, "no ceph-rmds"):
            g4.require_g4_daemon([])


class LocalDirectoryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="g4-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def test_a_local_directory_is_not_a_cephfs_mount(self):
        with self.assertRaises(g4.NotCephfs):
            g4.exercise(self.root)
        self.assertEqual(list(self.root.iterdir()), [])

    def test_a_symlink_root_is_refused(self):
        link = self.root / "link"
        target = self.root / "dir"
        target.mkdir()
        link.symlink_to(target, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "real directory"):
            g4.exercise(link)


class LiveMountTests(unittest.TestCase):
    def test_g4_mount_runs_the_scenario(self):
        mount = os.environ.get("G4_MOUNT")
        if not mount:
            self.skipTest("set G4_MOUNT to a CephFS mount of the target-g4 daemon")
        g4.exercise(mount)


if __name__ == "__main__":
    unittest.main()
