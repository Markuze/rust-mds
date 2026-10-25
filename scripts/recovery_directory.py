"""Small no-follow directory boundaries for recovery workload I/O."""
import os
import stat


def open_directory(path, *, dir_fd=None):
    return os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=dir_fd)


def validate_directory(path, fd, *, dir_fd=None):
    """The named directory must still be the retained physical directory."""
    named = os.stat(path, dir_fd=dir_fd, follow_symlinks=False)
    opened = os.fstat(fd)
    if (not stat.S_ISDIR(named.st_mode) or not stat.S_ISDIR(opened.st_mode) or
            (named.st_dev, named.st_ino) != (opened.st_dev, opened.st_ino)):
        raise ValueError("workload directory identity changed or became a symlink")
    return opened
