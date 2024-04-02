use core::ops::{Index, IndexMut};

use crate::TrapFrameArgs;

/// Saved registers when a trap (interrupt or exception) occurs.#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TrapFrame {
    pub regs: [usize; 31],
    pub sp: usize,
    pub elr: usize,
    pub spsr: usize,
    pub tpidr: usize,
}

impl TrapFrame {
    // 创建上下文信息
    #[inline]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    #[inline]
    pub fn args(&self) -> [usize; 6] {
        [
            self.regs[0],
            self.regs[1],
            self.regs[2],
            self.regs[3],
            self.regs[4],
            self.regs[5],
        ]
    }

    #[inline]
    pub fn syscall_ok(&mut self) {}
}

impl Index<TrapFrameArgs> for TrapFrame {
    type Output = usize;

    fn index(&self, index: TrapFrameArgs) -> &Self::Output {
        match index {
            TrapFrameArgs::SEPC => &self.elr,
            TrapFrameArgs::RA => &self.regs[30],
            TrapFrameArgs::SP => &self.sp,
            TrapFrameArgs::RET => &self.regs[0],
            TrapFrameArgs::ARG0 => &self.regs[0],
            TrapFrameArgs::ARG1 => &self.regs[1],
            TrapFrameArgs::ARG2 => &self.regs[2],
            TrapFrameArgs::TLS => &self.tpidr,
            TrapFrameArgs::SYSCALL => &self.regs[8],
        }
    }
}

impl IndexMut<TrapFrameArgs> for TrapFrame {
    fn index_mut(&mut self, index: TrapFrameArgs) -> &mut Self::Output {
        match index {
            TrapFrameArgs::SEPC => &mut self.elr,
            TrapFrameArgs::RA => &mut self.regs[30],
            TrapFrameArgs::SP => &mut self.sp,
            TrapFrameArgs::RET => &mut self.regs[0],
            TrapFrameArgs::ARG0 => &mut self.regs[0],
            TrapFrameArgs::ARG1 => &mut self.regs[1],
            TrapFrameArgs::ARG2 => &mut self.regs[2],
            TrapFrameArgs::TLS => &mut self.tpidr,
            TrapFrameArgs::SYSCALL => &mut self.regs[8],
        }
    }
}
