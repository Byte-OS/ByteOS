mod consts;
mod context;
mod gdt;
mod idt;
mod interrupt;
mod multiboot;
mod page_table;
mod pic;
mod sigtrx;
mod uart;

use ::multiboot::information::MemoryType;
pub use consts::*;
pub use context::Context;
pub use interrupt::*;
pub use multiboot::switch_to_kernel_page_table;
pub use page_table::*;
pub use uart::*;

use x86_64::instructions::port::PortWriteOnly;

use crate::{x86_64::multiboot::use_multiboot, ArchInterface};

#[percpu::def_percpu]
static CPU_ID: usize = 1;

pub fn shutdown() -> ! {
    unsafe { PortWriteOnly::new(0x604).write(0x2000u16) };

    loop {}
}

fn rust_tmp_main(magic: usize, mboot_ptr: usize) {
    crate::clear_bss();
    idt::init();
    pic::init();
    sigtrx::init();
    ArchInterface::init_logging();
    // Init allocator
    allocator::init();
    percpu::init(1);
    percpu::set_local_thread_pointer(0);
    gdt::init();
    interrupt::init_syscall();

    info!(
        "TEST CPU ID: {}  ptr: {:#x}",
        CPU_ID.read_current(),
        unsafe { CPU_ID.current_ptr() } as usize
    );
    CPU_ID.write_current(345);
    info!(
        "TEST CPU ID: {}  ptr: {:#x}",
        CPU_ID.read_current(),
        unsafe { CPU_ID.current_ptr() } as usize
    );

    info!("magic: {:#x}, mboot_ptr: {:#x}", magic, mboot_ptr);

    if let Some(mboot) = use_multiboot(mboot_ptr as _) {
        mboot
            .boot_loader_name()
            .inspect(|x| info!("bootloader: {}", x));
        mboot
            .command_line()
            .inspect(|x| info!("command_line: {}", x));
        if mboot.has_memory_map() {
            mboot
                .memory_regions()
                .unwrap()
                .filter(|x| x.memory_type() == MemoryType::Available)
                .for_each(|x| {
                    let start = x.base_address() as usize | VIRT_ADDR_START;
                    let end = x.length() as usize | VIRT_ADDR_START;
                    crate::ArchInterface::add_memory_region(start, end);
                });
        }
    }

    ArchInterface::prepare_drivers();

    crate::ArchInterface::main(0);

    shutdown()
}
