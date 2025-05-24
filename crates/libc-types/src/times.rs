//! This module provides the `libc` types for TIMES (time management).
//!
//!

/// 程序运行的时间
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/times.h#L11>
#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub struct TMS {
    /// 进程执行用户代码的时间
    pub utime: u64,
    /// 进程执行内核代码的时间
    pub stime: u64,
    /// 子进程执行用户代码的时间
    pub cutime: u64,
    /// 子进程执行内核代码的时间
    pub cstime: u64,
}
