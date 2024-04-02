use core::ops::{Index, IndexMut};

use crate::TrapFrameArgs;

/// Saved registers when a trap (interrupt or exception) occurs.
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TrapFrame {
    /// General Registers
    pub regs: [usize; 32],
    /// Pre-exception Mode information
    pub prmd: usize,
    /// Exception Return Address
    pub era: usize,
}

impl TrapFrame {
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

impl TrapFrame {
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

impl Index<TrapFrameArgs> for TrapFrame {
    type Output = usize;

    fn index(&self, index: TrapFrameArgs) -> &Self::Output {
        match index {
            TrapFrameArgs::SEPC => &self.era,
            TrapFrameArgs::RA => &self.regs[1],
            TrapFrameArgs::SP => &self.regs[3],
            TrapFrameArgs::RET => &self.regs[4],
            TrapFrameArgs::ARG0 => &self.regs[4],
            TrapFrameArgs::ARG1 => &self.regs[5],
            TrapFrameArgs::ARG2 => &self.regs[6],
            TrapFrameArgs::TLS => &self.regs[2],
            TrapFrameArgs::SYSCALL => &self.regs[11],
        }
    }
}

impl IndexMut<TrapFrameArgs> for TrapFrame {
    fn index_mut(&mut self, index: TrapFrameArgs) -> &mut Self::Output {
        match index {
            TrapFrameArgs::SEPC => &mut self.era,
            TrapFrameArgs::RA => &mut self.regs[1],
            TrapFrameArgs::SP => &mut self.regs[3],
            TrapFrameArgs::RET => &mut self.regs[4],
            TrapFrameArgs::ARG0 => &mut self.regs[4],
            TrapFrameArgs::ARG1 => &mut self.regs[5],
            TrapFrameArgs::ARG2 => &mut self.regs[6],
            TrapFrameArgs::TLS => &mut self.regs[2],
            TrapFrameArgs::SYSCALL => &mut self.regs[11],
        }
    }
}
