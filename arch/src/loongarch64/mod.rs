mod boot;
mod console;
mod consts;
mod context;
mod page_table;
mod timer;
mod trap;

pub use console::{console_getchar, console_putchar};
pub use consts::*;
pub use context::Context;
pub use page_table::*;
pub use timer::get_time;
pub use trap::{enable_external_irq, enable_irq, init_interrupt, run_user_task};

use crate::{clear_bss, ArchInterface};

pub fn rust_tmp_main(hart_id: usize) {
    clear_bss();
    ArchInterface::init_logging();
    allocator::init();
    trap::set_trap_vector_base();

    ArchInterface::add_memory_region(
        VIRT_ADDR_START | 0x9000_0000,
        VIRT_ADDR_START | (0x9000_0000 + 0x2000_0000),
    );
    info!("hart_id: {}", hart_id);
    unsafe {
        core::arch::asm!("break 2");
    }
    ArchInterface::prepare_drivers();

    shutdown();
}

pub fn time_to_usec(_t: usize) -> usize {
    todo!("time to usec")
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
