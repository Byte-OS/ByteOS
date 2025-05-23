// use crate::types::SigSet;

// use super::UStack;

// #[repr(C)]
// #[derive(Debug, Clone)]
// pub struct UContext {
//     pub flags: usize,  // 0
//     pub link: usize,   // 1
//     pub stack: UStack, // 2
//     pub gregs: [usize; 32],
//     pub sig_mask: SigSet, // sigmask
//     pub _pad: [u64; 15],  // sigmask extend
//     pub __fpregs_mem: [u64; 64],
// }
