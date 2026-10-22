"""Fault adapters around real scandir entries. No filesystem or process mutations."""
import errno
import os
from pathlib import Path


def scandir_with_type_error(real_scandir, directory):
    class Entry:
        def __init__(self, entry):
            self.entry = entry

        def __getattr__(self, name):
            return getattr(self.entry, name)

        def is_dir(self, *args, **kwargs):
            if Path(self.entry.path) == directory:
                raise OSError(errno.EIO, os.strerror(errno.EIO), self.entry.path)
            return self.entry.is_dir(*args, **kwargs)

    class Scan:
        def __init__(self, path):
            self.scan = real_scandir(path)

        def __enter__(self):
            self.scan.__enter__()
            return self

        def __exit__(self, *args):
            return self.scan.__exit__(*args)

        def __iter__(self):
            return self

        def __next__(self):
            return Entry(next(self.scan))

    return Scan
