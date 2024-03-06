use crate::{PTEFlags, VirtAddr};
use core::arch::asm;

#[link_section = ".data.prepage"]
static mut BOOT_PT_L1: [usize; 512] = [0; 512];

#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    todo!("flush_tlb")
}
