//! This module provides the `libc` types for FCNTL (file control).

use num_enum::TryFromPrimitive;

/// 当前目录的文件描述符
pub const AT_FDCWD: isize = -100;

/// 文件描述符控制命令
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/fcntl.h#L22>
/// TODO: 根据不同的平台实现不同的命令
#[repr(u32)]
#[derive(Debug, Clone, PartialEq, TryFromPrimitive)]
pub enum FcntlCmd {
    /// dup
    DUPFD = 0,
    /// get close_on_exec
    GETFD = 1,
    /// set/clear close_on_exec
    SETFD = 2,
    /// get file->f_flags
    GETFL = 3,
    /// set file->f_flags
    SETFL = 4,
    /// Get record locking info.
    GETLK = 5,
    /// Set record locking info (non-blocking).
    SETLK = 6,
    /// Set record locking info (blocking).
    SETLKW = 7,
    /// like F_DUPFD, but additionally set the close-on-exec flag
    DUPFDCLOEXEC = 0x406,
}
