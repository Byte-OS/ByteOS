//! This module provides the `libc` types for libc internal use.
//!
//!

use crate::types::SigSet;

/// 信号处理函数的结构体（对应 C 的 `struct sigaction`）
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/src/internal/ksigaction.h#L6>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SigAction {
    /// 信号处理函数指针，类似于 C 中的 void (*sa_handler)(int);
    /// 当信号发生时将调用此函数。也可以是特殊值，如 SIG_IGN 或 SIG_DFL。
    pub handler: usize,
    /// 标志位，用于指定处理行为，如 SA_RESTART、SA_NOCLDSTOP 等。
    /// 对应 C 中的 int sa_flags;
    pub flags: usize,
    /// 系统调用的恢复函数指针，一般在使用自定义恢复机制时使用。
    /// 对应 C 中的 void (*sa_restorer)(void); 通常不使用，设为 0。
    pub restorer: usize,
    /// 一个信号集合，用于在处理该信号时阻塞的其他信号。
    /// 对应 C 中的 sigset_t sa_mask;
    pub mask: SigSet,
}

impl SigAction {
    /// 创建一个新的信号处理函数结构体，所有字段初始化为默认值。
    pub const fn empty() -> Self {
        Self {
            handler: 0,
            mask: SigSet::empty(),
            flags: 0,
            restorer: 0,
        }
    }
}
