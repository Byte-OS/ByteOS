use crate::types::SigSetExtended;

use super::UStack;

/// 信号处理上下文的结构体
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/x86_64/bits/signal.h#L97>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct UContext {
    /// 标志位，用于表示上下文的状态或其他标记
    pub flags: usize,
    /// 链接寄存器，保存返回地址或跳转地址
    pub link: usize,
    /// 栈，保存上下文的栈信息
    pub stack: UStack,
    /// 通用寄存器的上下文信息
    pub gregs: MContext,
    /// 信号掩码，用于记录哪些信号被屏蔽
    pub sig_mask: SigSetExtended,
    /// 浮点寄存器的内存表示
    pub __fpregs_mem: [u64; 64],
}

/// 存放信号处理上下文的机器寄存器的结构体
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/x86_64/bits/signal.h#L72>
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MContext {
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rbp: usize,
    pub rbx: usize,
    pub rdx: usize,
    pub rax: usize,
    pub rcx: usize,
    pub rsp: usize,
    pub rip: usize,
    pub eflags: usize,
    pub cs: u8,
    pub gs: u8,
    pub fs: u8,
    __pad0: u8,
    pub err: usize,
    pub trapno: usize,
    pub oldmask: usize,
    pub cr2: usize,
    pub fp_ptr: usize,
    __reserved1: [usize; 8],
}
