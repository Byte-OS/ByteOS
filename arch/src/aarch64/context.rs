use core::ops::{Index, IndexMut};

use crate::ContextArgs;

/// Saved registers when a trap (interrupt or exception) occurs.#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct Context {
    pub regs: [usize; 31],
    pub sp: usize,
    pub elr: usize,
    pub spsr: usize,
    pub tpidr: usize,
}

impl Context {
    // 创建上下文信息
    #[inline]
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

// impl ContextOps for Context {
//     #[inline]
//     fn set_sp(&mut self, sp: usize) {
//         self.sp = sp
//     }

//     #[inline]
//     fn sp(&self) -> usize {
//         self.sp
//     }
//     #[inline]
//     fn set_ra(&mut self, ra: usize) {
//         self.regs[30] = ra;
//     }

//     #[inline]
//     fn ra(&self) -> usize {
//         self.regs[30]
//     }

//     #[inline]
//     fn set_sepc(&mut self, sepc: usize) {
//         self.elr = sepc;
//     }

//     #[inline]
//     fn sepc(&self) -> usize {
//         self.elr
//     }

//     #[inline]
//     fn syscall_number(&self) -> usize {
//         self.regs[8]
//     }

//     #[inline]
//     fn args(&self) -> [usize; 6] {
//         [
//             self.regs[0],
//             self.regs[1],
//             self.regs[2],
//             self.regs[3],
//             self.regs[4],
//             self.regs[5],
//         ]
//     }

//     #[inline]
//     fn syscall_ok(&mut self) {}

//     fn set_ret(&mut self, ret: usize) {
//         self.regs[0] = ret;
//     }

//     fn set_arg0(&mut self, ret: usize) {
//         self.regs[0] = ret;
//     }

//     fn set_arg1(&mut self, ret: usize) {
//         self.regs[1] = ret;
//     }

//     fn set_arg2(&mut self, ret: usize) {
//         self.regs[2] = ret;
//     }

//     #[inline]
//     fn set_tls(&mut self, tls: usize) {
//         self.tpidr = tls
//     }
// }

impl Context {
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

impl Index<ContextArgs> for Context {
    type Output = usize;

    fn index(&self, index: ContextArgs) -> &Self::Output {
        match index {
            ContextArgs::SEPC => &self.elr,
            ContextArgs::RA => &self.regs[30],
            ContextArgs::SP => &self.sp,
            ContextArgs::RET => &self.regs[0],
            ContextArgs::ARG0 => &self.regs[0],
            ContextArgs::ARG1 => &self.regs[1],
            ContextArgs::ARG2 => &self.regs[2],
            ContextArgs::TLS => &self.tpidr,
            ContextArgs::SYSCALL => &self.regs[8],
        }
    }
}

impl IndexMut<ContextArgs> for Context {
    fn index_mut(&mut self, index: ContextArgs) -> &mut Self::Output {
        match index {
            ContextArgs::SEPC => &mut self.elr,
            ContextArgs::RA => &mut self.regs[30],
            ContextArgs::SP => &mut self.sp,
            ContextArgs::RET => &mut self.regs[0],
            ContextArgs::ARG0 => &mut self.regs[0],
            ContextArgs::ARG1 => &mut self.regs[1],
            ContextArgs::ARG2 => &mut self.regs[2],
            ContextArgs::TLS => &mut self.tpidr,
            ContextArgs::SYSCALL => &mut self.regs[8],
        }
    }
}
