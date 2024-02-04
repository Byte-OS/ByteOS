use x86::controlregs;

use crate::{PhysAddr, PhysPage, VirtAddr, PAGE_FRAME_BASE};

pub struct PageTable(usize);

/// ppn convert, 如果在高半核空间
pub const fn ppn_c(ppn: PhysPage) -> PhysPage {
    PhysPage(ppn.0 | (PAGE_FRAME_BASE >> 12))
}

/// paddr convert, 如果在高半核空间
pub fn paddr_c(paddr: PhysAddr) -> PhysAddr {
    assert!(paddr.0 < PAGE_FRAME_BASE);
    PhysAddr(paddr.0 + PAGE_FRAME_BASE)
}

/// paddr number convert, 如果在高半核空间
pub fn paddr_cn(paddr: usize) -> usize {
    assert!(paddr < PAGE_FRAME_BASE);
    paddr + PAGE_FRAME_BASE
}

/// 虚拟地址转物理地址
pub fn virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    // current_page_table().virt_to_phys(vaddr)
    todo!()
}

#[inline]
pub fn current_page_table() -> PageTable {
    // PhysAddr::new(unsafe { controlregs::cr3() } as usize).align_down_4k()
    todo!()
}
