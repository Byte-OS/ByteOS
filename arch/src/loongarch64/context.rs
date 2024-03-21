use core::ops::{Index, IndexMut};

use crate::ContextArgs;

/// Saved registers when a trap (interrupt or exception) occurs.
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct Context {
    /// General Registers
    pub regs: [usize; 32],
    /// Pre-exception Mode information
    pub prmd: usize,
    /// Exception Return Address
    pub era: usize,
}

impl Context {
    // 创建上下文信息
    #[inline]
    pub fn new() -> Self {
        Self {
            // bit 1:0 PLV
            // bit 2 PIE
            // bit 3 PWE
            prmd: (0b0111),
            ..Default::default()
        }
    }
}

impl Context {
    pub fn syscall_ok(&mut self) {
        self.era += 4;
    }

    #[inline]
    pub fn args(&self) -> [usize; 6] {
        [
            self.regs[4],
            self.regs[5],
            self.regs[6],
            self.regs[7],
            self.regs[8],
            self.regs[9],
        ]
    }
}

impl Index<ContextArgs> for Context {
    type Output = usize;

    fn index(&self, index: ContextArgs) -> &Self::Output {
        match index {
            ContextArgs::SEPC => &self.era,
            ContextArgs::RA => &self.regs[1],
            ContextArgs::SP => &self.regs[3],
            ContextArgs::RET => &self.regs[4],
            ContextArgs::ARG0 => &self.regs[4],
            ContextArgs::ARG1 => &self.regs[5],
            ContextArgs::ARG2 => &self.regs[6],
            ContextArgs::TLS => &self.regs[2],
            ContextArgs::SYSCALL => &self.regs[11],
        }
    }
}

impl IndexMut<ContextArgs> for Context {
    fn index_mut(&mut self, index: ContextArgs) -> &mut Self::Output {
        match index {
            ContextArgs::SEPC => &mut self.era,
            ContextArgs::RA => &mut self.regs[1],
            ContextArgs::SP => &mut self.regs[3],
            ContextArgs::RET => &mut self.regs[4],
            ContextArgs::ARG0 => &mut self.regs[4],
            ContextArgs::ARG1 => &mut self.regs[5],
            ContextArgs::ARG2 => &mut self.regs[6],
            ContextArgs::TLS => &mut self.regs[2],
            ContextArgs::SYSCALL => &mut self.regs[11],
        }
    }
}
