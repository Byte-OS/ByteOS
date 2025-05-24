use crate::types::SigSetExtended;

use super::UStack;

/// 存放信号处理上下文的机器寄存器的结构体
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/loongarch64/bits/signal.h#L52>
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MContext {
    pub pc: usize,
    pub gregs: [usize; 32],
    pub gflags: u32,
    pub fcsr: u32,
    pub scr: [usize; 4],
    pub fregs: [usize; 32], // _extcontext
    _reserved: [usize; 512],
}

/// 用户上下文结构体（用于信号处理）
/// 对应 Linux 中的 `ucontext_t` 结构，用于保存进程的执行状态。
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/loongarch64/bits/signal.h#L44>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct UContext {
    /// 上下文标志，用于指定上下文信息的种类。
    pub flags: usize,
    /// 指向链接的上下文（如 `setcontext` 返回后恢复的上下文）。
    pub link: usize,
    /// 栈信息（指向 `stack_t` 结构，表示信号处理时使用的栈）。
    pub stack: UStack,
    /// 信号屏蔽字，表示处理该信号时应屏蔽的其他信号。
    pub sig_mask: SigSetExtended,
    /// 用于对齐填充，确保结构体与内核兼容。
    _pad: u64,
    /// 通用寄存器和浮点上下文等寄存器状态（封装在 `MContext` 中）。
    pub regs: MContext,
}
