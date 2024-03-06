mod boot;
mod consts;
mod context;
mod page_table;
mod pl011;
mod timer;
mod trap;

use alloc::vec::Vec;
pub use consts::*;
pub use context::Context;
use fdt::Fdt;
pub use page_table::*;
pub use pl011::{console_getchar, console_putchar};
pub use timer::get_time;
pub use trap::{enable_external_irq, enable_irq, init_interrupt, trap_pre_handle, user_restore};

use crate::{clear_bss, ArchInterface};

pub fn rust_tmp_main(hart_id: usize, device_tree: usize) {
    clear_bss();
    allocator::init();
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
    todo!("shutdown")
}
