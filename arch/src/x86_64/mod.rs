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

use ::multiboot::information::MemoryType;
pub use consts::*;
pub use context::Context;
pub use interrupt::*;
pub use page_table::*;
pub use uart::*;
pub use entry::switch_to_kernel_page_table;

use x86_64::instructions::port::PortWriteOnly;

use crate::x86_64::multiboot::use_multiboot;

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

    shutdown()
}

