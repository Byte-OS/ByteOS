mod consts;
mod context;
mod entry;
mod idt;
mod interrupt;
mod multiboot;
mod page_table;
mod pic;
mod sigtrx;
mod trap;
mod uart;

use core::mem::transmute;

use ::multiboot::information::MemoryType;
pub use consts::*;
pub use context::Context;
pub use interrupt::*;
pub use page_table::*;
pub use uart::*;
use x86::bits64::paging::{PAddr, PDPTEntry, PDPTFlags, PML4Entry, PML4Flags, PAGE_SIZE_ENTRIES, PDPT, PML4};
use x86_64::instructions::port::PortWriteOnly;

use crate::x86_64::multiboot::use_multiboot;

#[link_section = ".data.prepage.entry"]
static KERNEL_PDPT: PDPT = {
    let mut arr: PDPT = [PDPTEntry(0); PAGE_SIZE_ENTRIES];
    // 0x00000000_80000000 -> 0x80000000 (1G)
    // arr[0] = PDPTEntry::new(PAddr(0x0), PDPTFlags::P | PDPTFlags::RW | PDPTFlags::PS);
    // arr[1] = PDPTEntry::new(PAddr(0x40000000), PDPTFlags::P | PDPTFlags::RW | PDPTFlags::PS);
    // arr[2] = PDPTEntry::new(PAddr(0x80000000), PDPTFlags::P | PDPTFlags::RW | PDPTFlags::PS);
    // arr[3] = PDPTEntry::new(PAddr(0xc0000000), PDPTFlags::P | PDPTFlags::RW | PDPTFlags::PS);
    arr[0] = PDPTEntry(0x0 | 0x83);
    arr[1] = PDPTEntry(0x40000000 | 0x83);
    arr[2] = PDPTEntry(0x80000000 | 0x83);
    arr[3] = PDPTEntry(0xc0000000 | 0x83);
    arr
};

// #[link_section = ".data.prepage.entry"]
// static PAGE_TABLE: PML4 = {
//     let mut arr: PML4 = [PML4Entry(0); PAGE_SIZE_ENTRIES];

//     // arr[2] = PTE::from_addr(0x8000_0000, PTEFlags::ADVRWX);
//     // arr[0x100] = PTE::from_addr(0x0000_0000, PTEFlags::ADGVRWX);
//     // arr[0x101] = PTE::from_addr(0x4000_0000, PTEFlags::ADGVRWX);
//     // arr[0x102] = PTE::from_addr(0x8000_0000, PTEFlags::ADGVRWX);
//     // arr[0x106] = PTE::from_addr(0x8000_0000, PTEFlags::ADVRWX);
//     // arr[0] = PML4Entry::new(PAddr(KERNEL_PDPT.as_ptr() as u64 - VIRT_ADDR_START as u64), PML4Flags::P | PML4Flags::RW);
//     let ptr = &KERNEL_PDPT as *const [PDPTEntry; PAGE_SIZE_ENTRIES] as *const PDPTEntry;
//     let paddr: u64 = unsafe { transmute(ptr.sub(VIRT_ADDR_START)) };
//     arr[0] = PML4Entry(paddr | 3);
//     arr
// };

pub fn shutdown() -> ! {
    unsafe { PortWriteOnly::new(0x604).write(0x2000u16) };

    loop {}
}

fn rust_tmp_main(magic: usize, mboot_ptr: usize) {
    crate::clear_bss();
    crate::prepare_init();
    idt::init();
    pic::init();

    info!("magic: {:#x}, mboot_ptr: {:#x}", magic, mboot_ptr);

    if let Some(mboot) = use_multiboot(mboot_ptr as _) {
        if mboot.has_memory_map() {
            info!("has memory map");
            mboot
                .memory_regions()
                .unwrap()
                .filter(|x| x.memory_type() == MemoryType::Available)
                .for_each(|x| {
                    let start = x.base_address() as usize | VIRT_ADDR_START;
                    let end = x.length() as usize | VIRT_ADDR_START;
                    info!(
                        "memory region: {:#x} length: {:#x}, type: {:#x?}",
                        start,
                        end,
                        x.memory_type()
                    );
                    crate::ArchInterface::add_memory_region(start, end);
                });
        }
    }

    crate::ArchInterface::main(0, 0);
}

