pub mod sigtrx;
mod sv39;

use core::arch::riscv64::sfence_vma;

pub use sv39::*;

use crate::VirtAddr;

#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    unsafe {
        if let Some(vaddr) = vaddr {
            // TIPS: flush tlb, tlb addr: 0-47: ppn, otherwise tlb asid
            sfence_vma(vaddr.addr(), 0);
        } else {
            // flush the entire TLB
            riscv::asm::sfence_vma_all();
        }
    }
}
