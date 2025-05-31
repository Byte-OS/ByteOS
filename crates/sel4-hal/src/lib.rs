#![no_std]

pub mod addr;

use core::ops::{Index, IndexMut};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TrapFrame {}

impl TrapFrame {
    pub const fn new() -> Self {
        todo!()
    }
}

impl Index<TrapFrameArgs> for TrapFrame {
    type Output = usize;

    fn index(&self, index: TrapFrameArgs) -> &Self::Output {
        todo!()
        // match index {
        //     TrapFrameArgs::SEPC => &self.elr,
        //     TrapFrameArgs::RA => &self.regs[30],
        //     TrapFrameArgs::SP => &self.sp,
        //     TrapFrameArgs::RET => &self.regs[0],
        //     TrapFrameArgs::ARG0 => &self.regs[0],
        //     TrapFrameArgs::ARG1 => &self.regs[1],
        //     TrapFrameArgs::ARG2 => &self.regs[2],
        //     TrapFrameArgs::TLS => &self.tpidr,
        //     TrapFrameArgs::SYSCALL => &self.regs[8],
        // }
    }
}

impl IndexMut<TrapFrameArgs> for TrapFrame {
    fn index_mut(&mut self, index: TrapFrameArgs) -> &mut Self::Output {
        todo!()
        // match index {
        //     TrapFrameArgs::SEPC => &mut self.elr,
        //     TrapFrameArgs::RA => &mut self.regs[30],
        //     TrapFrameArgs::SP => &mut self.sp,
        //     TrapFrameArgs::RET => &mut self.regs[0],
        //     TrapFrameArgs::ARG0 => &mut self.regs[0],
        //     TrapFrameArgs::ARG1 => &mut self.regs[1],
        //     TrapFrameArgs::ARG2 => &mut self.regs[2],
        //     TrapFrameArgs::TLS => &mut self.tpidr,
        //     TrapFrameArgs::SYSCALL => &mut self.regs[8],
        // }
    }
}

/// Trap Frame Arg Type
///
/// Using this by Index and IndexMut trait bound on TrapFrame
#[derive(Debug)]
pub enum TrapFrameArgs {
    SEPC,
    RA,
    SP,
    RET,
    ARG0,
    ARG1,
    ARG2,
    TLS,
    SYSCALL,
}

#[macro_export]
macro_rules! bit {
    ($x: expr) => {
        (1 << $x)
    };
}

bitflags::bitflags! {
    /// Mapping flags for page table.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MappingFlags: u64 {
        /// Persent
        const P = bit!(0);
        /// User Accessable Flag
        const U = bit!(1);
        /// Readable Flag
        const R = bit!(2);
        /// Writeable Flag
        const W = bit!(3);
        /// Executeable Flag
        const X = bit!(4);
        /// Accessed Flag
        const A = bit!(5);
        /// Dirty Flag, indicating that the page was written
        const D = bit!(6);
        /// Global Flag
        const G = bit!(7);
        /// Device Flag, indicating that the page was used for device memory
        const Device = bit!(8);
        /// Cache Flag, indicating that the page will be cached
        const Cache = bit!(9);

        /// Read | Write | Executeable Flags
        const RWX = Self::R.bits() | Self::W.bits() | Self::X.bits();
        /// User | Read | Write Flags
        const URW = Self::U.bits() | Self::R.bits() | Self::W.bits();
        /// User | Read | Executeable Flags
        const URX = Self::U.bits() | Self::R.bits() | Self::X.bits();
        /// User | Read | Write | Executeable Flags
        const URWX = Self::URW.bits() | Self::X.bits();
    }
}
