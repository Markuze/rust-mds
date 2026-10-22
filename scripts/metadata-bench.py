#!/usr/bin/env python3
"""Measure one client on an existing mount. Run as root on root-owned mounts."""
import argparse
import json
import os
from pathlib import Path
import platform
import statistics
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mount", type=Path)
    parser.add_argument("--label", required=True, choices=("cpp", "rust"))
    parser.add_argument("--count", type=int, default=100)
    args = parser.parse_args()
    if args.count < 1:
        parser.error("count must be positive")
    if not os.path.ismount(args.mount):
        parser.error("path must be a mount point")
    root = Path(tempfile.mkdtemp(prefix="rmds-bench-", dir=args.mount))
    rows = {}
    dfd = os.open(root, os.O_RDONLY | os.O_DIRECTORY)
    try:
        for mode in ("create_close", "create_fsync_dir", "write_fsync"):
            times = []
            for i in range(args.count):
                name = root / f"{mode}-{i}"
                start = time.monotonic_ns()
                fd = os.open(name, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
                try:
                    if mode == "write_fsync":
                        if os.write(fd, b"x" * 4096) != 4096:
                            raise OSError("short benchmark write")
                    if mode != "create_close":
                        os.fsync(fd)
                finally:
                    os.close(fd)
                if mode == "create_fsync_dir":
                    os.fsync(dfd)
                times.append((time.monotonic_ns() - start) / 1000)
            ordered = sorted(times)
            rows[mode] = {"count": len(times), "mean_us": statistics.mean(times),
                          "p50_us": statistics.median(times),
                          "p99_us": ordered[min(len(times) - 1, int(len(times) * .99))],
                          "samples_us": times}
        print(json.dumps({"label": args.label, "kernel": platform.release(),
                          "note": "single client; warm cache; create_close can include C++ early replies; Rust is not yet durable",
                          "results": rows}))
    finally:
        os.close(dfd)
        for child in root.iterdir():
            child.unlink()
        root.rmdir()


if __name__ == "__main__":
    main()
