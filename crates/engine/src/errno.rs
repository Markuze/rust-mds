//! Errors of engine operations, mapped to the Linux errno values that MDS
//! replies carry (`MClientReply::result` is a negative errno,
//! `src/messages/MClientReply.h`).

use std::fmt;

/// An operation error. `as_neg()` gives the value for the reply's `result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Errno {
    EPERM = 1,
    ENOENT = 2,
    EINTR = 4,
    EIO = 5,
    EBADF = 9,
    EAGAIN = 11,
    ENOMEM = 12,
    EACCES = 13,
    EBUSY = 16,
    EEXIST = 17,
    EXDEV = 18,
    ENOTDIR = 20,
    EISDIR = 21,
    EINVAL = 22,
    EFBIG = 27,
    ENOSPC = 28,
    EROFS = 30,
    EMLINK = 31,
    EDOM = 33,
    ERANGE = 34,
    EDEADLK = 35,
    ENAMETOOLONG = 36,
    ENOLCK = 37,
    ENOSYS = 38,
    ENOTEMPTY = 39,
    ENODATA = 61,
    EOPNOTSUPP = 95,
    ESTALE = 116,
    EDQUOT = 122,
}

impl Errno {
    /// The positive errno value.
    pub fn code(self) -> i32 {
        self as i32
    }

    /// The negative errno value an MDS reply carries.
    pub fn as_neg(self) -> i32 {
        -(self as i32)
    }

    /// Map a positive or negative Linux errno back to an `Errno`.
    pub fn from_code(code: i32) -> Option<Errno> {
        use Errno::*;
        let all = [
            EPERM,
            ENOENT,
            EINTR,
            EIO,
            EBADF,
            EAGAIN,
            ENOMEM,
            EACCES,
            EBUSY,
            EEXIST,
            EXDEV,
            ENOTDIR,
            EISDIR,
            EINVAL,
            EFBIG,
            ENOSPC,
            EROFS,
            EMLINK,
            EDOM,
            ERANGE,
            EDEADLK,
            ENAMETOOLONG,
            ENOLCK,
            ENOSYS,
            ENOTEMPTY,
            ENODATA,
            EOPNOTSUPP,
            ESTALE,
            EDQUOT,
        ];
        let c = code.abs();
        all.into_iter().find(|e| e.code() == c)
    }
}

impl fmt::Display for Errno {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} ({})", self, self.as_neg())
    }
}

impl std::error::Error for Errno {}

/// Result type of engine-internal operations.
pub type EResult<T> = Result<T, Errno>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_values() {
        assert_eq!(Errno::ENOENT.as_neg(), -2);
        assert_eq!(Errno::ENOTEMPTY.code(), 39);
        assert_eq!(Errno::ENODATA.code(), 61);
        assert_eq!(Errno::ESTALE.code(), 116);
        assert_eq!(Errno::EDEADLK.code(), 35);
        assert_eq!(Errno::from_code(-39), Some(Errno::ENOTEMPTY));
        assert_eq!(Errno::from_code(4242), None);
    }
}
