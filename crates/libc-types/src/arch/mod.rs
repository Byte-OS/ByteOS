//! This module provides the `libc` types for each architecture.

#[cfg(target_arch = "aarch64")]
pub mod aarch64;
#[cfg(target_arch = "aarch64")]
pub use aarch64::{MContext, UContext};
#[cfg(target_arch = "loongarch64")]
pub mod loongarch64;
#[cfg(target_arch = "loongarch64")]
pub use loongarch64::{MContext, UContext};
#[cfg(target_arch = "riscv64")]
pub mod riscv64;
#[cfg(target_arch = "riscv64")]
pub use riscv64::{MContext, UContext};
#[cfg(target_arch = "x86_64")]
pub mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::{MContext, UContext};

use crate::types::TimeSpec;

bitflags! {
    /// 信号处理栈的标志位，控制备用信号栈（alternate signal stack）的行为。
    #[derive(Debug, Clone)]
    pub struct SignalStackFlags: u32 {
        /// 当前正在备用信号栈上执行（内核设置此位，用户态只读）。
        const ONSTACK = 1;
        /// 禁用备用信号栈（不会在该栈上调用信号处理函数）。
        const DISABLE = 2;
        /// 当信号处理程序在备用栈上返回时自动禁用备用栈（Linux 特有）。
        const AUTODISARM = 0x80000000;
    }
}

/// 备用信号栈（alternate signal stack）
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/x86_64/bits/signal.h#L91>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct UStack {
    /// 栈顶指针（备用信号栈的栈顶地址，通常是向下增长的内存区域）。
    /// 对应 C 中的 void *ss_sp;
    pub sp: usize,
    /// 标志位，表示备用栈的状态，比如是否启用、是否正在使用等。
    /// 对应 C 中的 int ss_flags;
    pub flags: SignalStackFlags,
    /// 栈的大小（以字节为单位），表示备用信号栈的长度。
    /// 对应 C 中的 size_t ss_size;
    pub size: usize,
}

bitflags! {
    /// 文件的状态信息，类似于 Linux 中的 `stat` 结构体。
    #[derive(Debug, Default, PartialEq)]
    pub struct StatMode: u32 {
        /// Null
        const NULL  = 0;
        /// Type
        const TYPE_MASK = 0o170000;
        /// FIFO
        const FIFO  = 0o010000;
        /// character device
        const CHAR  = 0o020000;
        /// directory
        const DIR   = 0o040000;
        /// block device
        const BLOCK = 0o060000;
        /// ordinary regular file
        const FILE  = 0o100000;
        /// symbolic link
        const LINK  = 0o120000;
        /// socket
        const SOCKET = 0o140000;

        /// Set-user-ID on execution.
        const SET_UID = 0o4000;
        /// Set-group-ID on execution.
        const SET_GID = 0o2000;

        /// Read, write, execute/search by owner.
        const OWNER_MASK = 0o700;
        /// Read permission, owner.
        const OWNER_READ = 0o400;
        /// Write permission, owner.
        const OWNER_WRITE = 0o200;
        /// Execute/search permission, owner.
        const OWNER_EXEC = 0o100;

        /// Read, write, execute/search by group.
        const GROUP_MASK = 0o70;
        /// Read permission, group.
        const GROUP_READ = 0o40;
        /// Write permission, group.
        const GROUP_WRITE = 0o20;
        /// Execute/search permission, group.
        const GROUP_EXEC = 0o10;

        /// Read, write, execute/search by others.
        const OTHER_MASK = 0o7;
        /// Read permission, others.
        const OTHER_READ = 0o4;
        /// Write permission, others.
        const OTHER_WRITE = 0o2;
        /// Execute/search permission, others.
        const OTHER_EXEC = 0o1;
    }
}

#[repr(C)]
#[derive(Debug, Default)]
#[cfg(not(target_arch = "x86_64"))]
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/aarch64/kstat.h#L1>
/// 文件的状态信息，类似于 Linux 中的 `stat` 结构体。
pub struct Stat {
    /// 设备号，表示文件所在设备的标识符。
    pub dev: u64,
    /// inode 号，表示文件在文件系统中的唯一标识符。
    pub ino: u64,
    /// 文件类型和访问权限，使用 `StatMode` 类型表示。
    pub mode: StatMode,
    /// 文件的硬链接数，表示指向该文件的链接数量。
    pub nlink: u32,
    /// 文件所有者的用户ID（UID）。
    pub uid: u32,
    /// 文件所有者的组ID（GID）。
    pub gid: u32,
    /// 设备文件的设备号，表示特殊设备文件的主设备号和次设备号。
    pub rdev: u64,
    /// 保留字段，用于对齐或将来扩展使用。
    pub __pad: u64,
    /// 文件的大小，以字节为单位。
    pub size: u64,
    /// 文件的块大小，文件系统为文件分配的块的大小。
    pub blksize: u32,
    /// 保留字段，用于对齐或将来扩展使用。
    pub __pad2: u32,
    /// 文件的占用块数，文件占用的实际磁盘块数。
    pub blocks: u64,
    /// 文件的最后访问时间（以 `TimeSpec` 表示）。
    pub atime: TimeSpec,
    /// 文件的最后修改时间（以 `TimeSpec` 表示）。
    pub mtime: TimeSpec,
    /// 文件的最后状态变更时间（以 `TimeSpec` 表示）。
    pub ctime: TimeSpec,
}

#[repr(C)]
#[derive(Debug, Default)]
#[cfg(target_arch = "x86_64")]
/// 文件的状态信息，类似于 Linux 中的 `stat` 结构体。
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/x86_64/bits/stat.h#L4>
/// 在 x86_64上, blksize_t 的大小为 8 字节, 而不是 4 字节，指向 long
pub struct Stat {
    /// 设备号，表示文件所在设备的标识符。
    pub dev: u64,
    /// inode 号，表示文件在文件系统中的唯一标识符。
    pub ino: u64,
    /// 文件的硬链接数，表示指向该文件的链接数量。
    pub nlink: u64,
    /// 文件类型和访问权限，使用 `StatMode` 类型表示。
    pub mode: StatMode,
    /// 文件所有者的用户ID（UID）。
    pub uid: u32,
    /// 文件所有者的组ID（GID）。
    pub gid: u32,
    /// 填充字段，确保结构体大小对齐。
    pub _pad0: u32,
    /// 设备文件的设备号，表示特殊设备文件的主设备号和次设备号。
    pub rdev: u64,
    /// 文件的大小，以字节为单位。
    pub size: u64,
    /// 文件的块大小，文件系统为文件分配的块的大小。
    pub blksize: u64,
    /// 文件的占用块数，文件占用的实际磁盘块数。
    pub blocks: u64,
    /// 文件的最后访问时间（以 `TimeSpec` 表示）。
    pub atime: TimeSpec,
    /// 文件的最后修改时间（以 `TimeSpec` 表示）。
    pub mtime: TimeSpec,
    /// 文件的最后状态变更时间（以 `TimeSpec` 表示）。
    pub ctime: TimeSpec,
}
