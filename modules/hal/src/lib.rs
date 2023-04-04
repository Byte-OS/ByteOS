#![no_std]

extern crate alloc;
pub mod interrupt;

/// test for page table, also a mini example for page table
#[test]
pub fn test_page_table() {
    let curr_page_table = current_page_table();
    let vaddr = 0x4000_3000 + VIRT_ADDR_START;
    let mut trackers = Vec::new();
    let mut alloc = || {
        let tracker = frame_alloc().expect("can't alloc page");
        let ppn = tracker.0;
        trackers.push(tracker);
        ppn
    };
    let ppn = alloc();
    curr_page_table.map(ppn, VirtPage::from_addr(vaddr), PTEFlags::VRWX, alloc);
    let paddr = curr_page_table.virt_to_phys(VirtAddr::new(vaddr));
    info!("page table vaddr: {:#X} to {}", vaddr, paddr);
}
