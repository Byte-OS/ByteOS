//! This module provides the `libc` types for Types.
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/alltypes.h.in>

use core::{cmp::Ordering, ops::Add};

/// IoVec structure
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/alltypes.h.in#L78>
#[repr(C)]
#[derive(Clone)]
pub struct IoVec {
    /// Base address of the buffer
    pub base: usize,
    /// Length of the buffer
    pub len: usize,
}

/// TimeVal structure
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/alltypes.h.in#L43>
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeVal {
    /// seconds, range in 0~999999999
    pub sec: usize,
    /// microseconds, range in 0~999999
    pub usec: usize,
}

impl Add for TimeVal {
    type Output = TimeVal;

    fn add(self, rhs: Self) -> Self::Output {
        let target = self.usec + rhs.usec;
        Self {
            sec: self.sec + rhs.sec + target / 1_000_000_000,
            usec: target % 1_000_000_000,
        }
    }
}

impl PartialOrd for TimeVal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.sec > other.sec {
            Some(Ordering::Greater)
        } else if self.sec < other.sec {
            Some(Ordering::Less)
        } else {
            if self.usec > other.usec {
                Some(Ordering::Greater)
            } else if self.usec < other.usec {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Equal)
            }
        }
    }
}

#[repr(C)]
/// 表示目录项（dirent）的结构体，用于读取目录内容（如 getdents64）
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/dirent.h#L5C1-L11C3>
pub struct Dirent64 {
    /// inode 号（索引节点号），唯一标识文件
    pub ino: u64,
    /// 偏移量，指向下一个目录项在目录流中的位置（用于遍历）
    pub off: i64,
    /// 当前目录项结构体的长度（包括文件名），单位是字节
    pub reclen: u16,
    /// 文件类型（如常规文件、目录、符号链接等）
    pub ftype: u8,
    /// 文件名（不定长，以空字符结尾，声明为 0 长度数组便于动态追加）
    pub name: [u8; 0],
}

#[repr(C)]
/// 文件系统状态结构体，用于 statfs 系统调用返回信息
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/statfs.h#L4>
pub struct StatFS {
    /// 文件系统的类型（magic number，用于标识如 ext4, tmpfs 等）
    pub ftype: u64,
    /// 最优传输块大小（用于文件系统的 I/O 操作优化）
    pub bsize: u64,
    /// 文件系统中数据块的总数量
    pub blocks: u64,
    /// 当前空闲的数据块数量（包括超级用户可用）
    pub bfree: u64,
    /// 普通用户可用的数据块数量（不包括超级用户保留）
    pub bavail: u64,
    /// 文件结点（i-node）总数，表示最多可创建的文件数量
    pub files: u64,
    /// 可用的文件结点数
    pub ffree: u64,
    /// 文件系统标识符（通常是一个唯一的 ID，用于区分不同的文件系统挂载点）
    pub fsid: u64,
    /// 支持的最大文件名长度（单位：字节）
    pub namelen: u64,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
/// 表示高精度时间的结构体（通常用于系统调用中的时间表示）
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/alltypes.h.in#L44>
/// TODO: 对比 MUSL 中的定义
pub struct TimeSpec {
    /// 秒（Seconds）
    pub sec: usize,
    /// 纳秒（Nanoseconds），有效范围为 0~999_999_999
    pub nsec: usize,
}

impl TimeSpec {
    /// 将 TimeSpec 转换为纳秒（nanoseconds）
    pub fn to_nsec(&self) -> usize {
        self.sec * 1_000_000_000 + self.nsec
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
/// 终端窗口大小结构体
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/alltypes.h.in#L80>
pub struct WinSize {
    /// 窗口的行数（以字符为单位）
    pub row: u16,
    /// 窗口的列数（以字符为单位）
    pub col: u16,
    /// 窗口的宽度（以像素为单位）
    pub xpixel: u16,
    /// 窗口的高度（以像素为单位）
    pub ypixel: u16,
}
