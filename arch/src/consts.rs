use core::mem::size_of;

use crate::TrapFrame;

/// Boot Stack Size.
/// TODO: reduce the boot stack size. Map stack in boot step.
pub const STACK_SIZE: usize = 0x8_0000;

/// The size of the trap frame(diffent in each architecture.).
pub const TRAPFRAME_SIZE: usize = size_of::<TrapFrame>();
