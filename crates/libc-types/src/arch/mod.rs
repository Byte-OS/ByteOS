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
