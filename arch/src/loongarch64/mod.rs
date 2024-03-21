mod boot;
mod console;
mod consts;
mod context;
mod page_table;
mod sigtrx;
mod timer;
mod trap;

pub use console::{console_getchar, console_putchar};
pub use consts::*;
pub use context::Context;
use loongarch64::register::euen;
pub use page_table::*;
pub use timer::{get_time, time_to_usec};
pub use trap::{enable_external_irq, enable_irq, init_interrupt, run_user_task};

use crate::{clear_bss, ArchInterface};

pub fn rust_tmp_main(hart_id: usize) {
    clear_bss();
    ArchInterface::init_logging();
    allocator::init();
    trap::set_trap_vector_base();
    sigtrx::init();

    ArchInterface::add_memory_region(
        VIRT_ADDR_START | 0x9000_0000,
        VIRT_ADDR_START | (0x9000_0000 + 0x2000_0000),
    );
    info!("hart_id: {}", hart_id);

    ArchInterface::prepare_drivers();

    // Enable floating point
    euen::set_fpe(true);

    ArchInterface::main(0);

    shutdown();
}

pub fn shutdown() -> ! {
    error!("shutdown!");
    loop {
        unsafe { loongarch64::asm::idle() };
    }
}
