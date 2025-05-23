//! This module provides the `libc` types for aarch64.
//!
//!

// #[repr(C)]
// #[derive(Debug, Clone)]
// pub struct UContext {
//     pub flags: usize,       // 0
//     pub link: usize,        // 1
//     pub stack: SignalStack, // 2
//     pub sig_mask: SigSet,   // 5
//     pub _pad: [u64; 15],    // mask
//     pub fault_address: usize,
//     pub regs: [usize; 31],
//     pub sp: usize,
//     pub pc: usize,
//     pub pstate: usize,
//     pub __reserved: usize,
// }

// impl UContext {
//     pub fn pc(&self) -> usize {
//         self.pc
//     }

//     pub fn set_pc(&mut self, v: usize) {
//         self.pc = v;
//     }

//     pub fn store_ctx(&mut self, ctx: &TrapFrame) {
//         self.regs = ctx.regs;
//     }
//     pub fn restore_ctx(&self, ctx: &mut TrapFrame) {
//         ctx.regs = self.regs;
//     }
// }
