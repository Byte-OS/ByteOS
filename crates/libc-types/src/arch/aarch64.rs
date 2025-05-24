//! This module provides the `libc` types for aarch64.
//!
//!

use crate::types::SigSetExtended;

use super::UStack;
/// 存放信号处理上下文的机器寄存器的结构体
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/aarch64/bits/signal.h#L18>
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MContext {
    pub fault_address: usize,
    pub gregs: [usize; 32],
    pub sp: usize,
    pub pc: usize,
    pub pstate: usize,
    pub __reserved: [u64; 66],
}

/// 信号处理上下文的结构体
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/aarch64/bits/signal.h#L99>
#[repr(C)]
#[derive(Debug, Clone)]
pub struct UContext {
    /// 标志位，用于表示上下文的状态或其他标记
    pub flags: usize,
    /// 链接寄存器，保存返回地址或跳转地址
    pub link: usize,
    /// 栈，保存上下文的栈信息
    pub stack: UStack,
    /// 信号掩码，用于记录哪些信号被屏蔽
    pub sig_mask: SigSetExtended,
    /// 机器寄存器的上下文信息
    pub regs: MContext,
}
