mod boot;
mod console;
mod consts;
mod context;
mod page_table;
mod timer;
mod trap;

use alloc::vec::Vec;
pub use console::{console_getchar, console_putchar};
pub use consts::*;
pub use context::Context;
use fdt::Fdt;
pub use page_table::*;
pub use timer::get_time;
pub use trap::{enable_external_irq, enable_irq, init_interrupt, trap_pre_handle, user_restore};

use crate::{clear_bss, ArchInterface};

pub fn rust_tmp_main(hart_id: usize) {
    clear_bss();
    ArchInterface::init_logging();
    allocator::init();

    ArchInterface::add_memory_region(
        VIRT_ADDR_START | 0x9000_0000,
        VIRT_ADDR_START | (0x9000_0000 + 0x2f00_0000),
    );
    info!("hart_id: {}", hart_id);

    ArchInterface::prepare_drivers();

    shutdown();
}

pub fn time_to_usec(_t: usize) -> usize {
    todo!("time to usec")
}

pub fn get_time_ms() -> usize {
    todo!("get_time_ms")
}

pub fn switch_to_kernel_page_table() {
    todo!("switch to kernel page table")
}

pub fn shutdown() -> ! {
    error!("shutdown!");
    loop {
        unsafe { loongarch64::asm::idle() };
    }
}
