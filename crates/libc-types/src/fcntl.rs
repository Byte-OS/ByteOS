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

#[cfg(any(
    target_arch = "riscv64",
    target_arch = "loongarch64",
    target_arch = "x86_64"
))]
bitflags! {
    /// 文件打开标志，对应 Linux 的 open(2) 系统调用选项。
    #[derive(Debug, Clone)]
    pub struct OpenFlags: usize {
        /// 只读（Read Only）
        const RDONLY      = 0;
        /// 只写（Write Only）
        const WRONLY      = 1;
        /// 读写（Read and Write）
        const RDWR        = 2;
        /// 访问模式掩码（Access mode mask）
        const ACCMODE     = 3;
        /// 文件不存在时创建（Create file if it does not exist）
        const CREAT       = 0o100;
        /// 与 O_CREAT 一起使用，文件存在时报错（Error if file exists）
        const EXCL        = 0o200;
        /// 不将设备设为控制终端（Do not make device a controlling terminal）
        const NOCTTY      = 0o400;
        /// 打开文件时清空内容（Truncate file to zero length）
        const TRUNC       = 0o1000;
        /// 每次写入都追加到末尾（Append on each write）
        const APPEND      = 0o2000;
        /// 非阻塞模式（Non-blocking I/O）
        const NONBLOCK    = 0o4000;
        /// 数据写入后立即同步（Synchronize data writes）
        const DSYNC       = 0o10000;
        /// 数据和元数据写入后同步（Synchronize all writes）
        const SYNC        = 0o4010000;
        /// 同步读操作（Same as O_SYNC）
        const RSYNC       = 0o4010000;
        /// 打开目标必须为目录（Fail if not a directory）
        const DIRECTORY   = 0o200000;
        /// 不跟随符号链接（Do not follow symlinks）
        const NOFOLLOW    = 0o400000;
        /// 执行 exec 时关闭（Close on exec）
        const CLOEXEC     = 0o2000000;
        /// 启用异步 I/O（Enable signal-driven I/O）
        const ASYNC       = 0o20000;
        /// 绕过页缓存直接 I/O（Direct disk access）
        const DIRECT      = 0o40000;
        /// 启用大文件支持（Enable large files）
        const LARGEFILE   = 0o100000;
        /// 不更新 atime（Do not update access time）
        const NOATIME     = 0o1000000;
        /// 仅解析路径，不打开目标（Open just the path）
        const PATH        = 0o10000000;
        /// 创建匿名临时文件（Unnamed temporary file）
        const TMPFILE     = 0o20200000;
    }

}

#[cfg(target_arch = "aarch64")]
bitflags! {
    /// 文件打开标志，对应 Linux 的 open(2) 系统调用选项。
    #[derive(Debug, Clone)]
    pub struct OpenFlags: usize {
        /// 只读（Read Only）
        const RDONLY = 0;
        /// 只写（Write Only）
        const WRONLY = 1;
        /// 读写（Read and Write）
        const RDWR = 2;
        /// 访问模式掩码（用于提取读写模式）
        const ACCMODE = 3;
        /// 文件不存在时创建（Create if not exist）
        const CREAT = 0o100;
        /// 与 O_CREAT 配合使用，文件存在时报错（Exclusive）
        const EXCL = 0o200;
        /// 不将设备设为控制终端（No controlling TTY）
        const NOCTTY = 0o400;
        /// 打开时截断为 0 长度（Truncate）
        const TRUNC = 0o1000;
        /// 写操作追加到文件末尾（Append）
        const APPEND = 0o2000;
        /// 非阻塞模式（Non-blocking）
        const NONBLOCK = 0o4000;
        /// 数据同步写入（Data sync）
        const DSYNC = 0o10000;
        /// 数据+元数据同步写入（Full sync）
        const SYNC = 0o4010000;
        /// 同步读操作（Same as O_SYNC）
        const RSYNC = 0o4010000;
        /// 必须是目录（Directory only）
        const DIRECTORY = 0o40000;
        /// 不跟随符号链接（No follow symlinks）
        const NOFOLLOW = 0o100000;
        /// 执行 exec 时自动关闭（Close-on-exec）
        const CLOEXEC = 0o2000000;
        /// 启用异步 I/O 信号（Async notify）
        const ASYNC = 0o20000;
        /// 直接 I/O，绕过缓存（Direct access）
        const DIRECT = 0o200000;
        /// 启用大文件支持（Large file）
        const LARGEFILE = 0o400000;
        /// 不更新访问时间（No atime update）
        const NOATIME = 0o1000000;
        /// 只解析路径，不打开目标（Path only）
        const PATH = 0o10000000;
        /// 创建临时匿名文件（Unnamed temporary file）
        const TMPFILE = 0o20040000;
    }
}
