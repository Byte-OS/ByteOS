mod boot;
mod consts;
mod context;
mod page_table;
mod pl011;
mod psci;
mod timer;
mod trap;

use aarch64_cpu::registers::Writeable;
use aarch64_cpu::{asm::barrier, registers::CPACR_EL1};
use alloc::vec::Vec;
pub use consts::*;
pub use context::Context;
use fdt::Fdt;
pub use page_table::*;
pub use pl011::{console_getchar, console_putchar};
pub use psci::system_off as shutdown;
pub use timer::get_time;
pub use trap::{enable_external_irq, enable_irq, init_interrupt, trap_pre_handle, user_restore};

use crate::{clear_bss, ArchInterface};

pub fn rust_tmp_main(hart_id: usize, device_tree: usize) {
    clear_bss();
    pl011::init_early();
    ArchInterface::init_logging();
    trap::init();
    allocator::init();

    let mut dt_buf = Vec::new();

    if device_tree != 0 {
        let fdt = unsafe { Fdt::from_ptr(device_tree as *const u8).unwrap() };

        dt_buf.extend_from_slice(unsafe {
            core::slice::from_raw_parts(device_tree as *const u8, fdt.total_size())
        });

        info!("There has {} CPU(s)", fdt.cpus().count());

        fdt.memory()
            .regions()
            .for_each(|x: fdt::standard_nodes::MemoryRegion| {
                info!(
                    "memory region {:#X} - {:#X}",
                    x.starting_address as usize,
                    x.starting_address as usize + x.size.unwrap()
                );

                ArchInterface::add_memory_region(
                    x.starting_address as usize | VIRT_ADDR_START,
                    (x.starting_address as usize + x.size.unwrap()) | VIRT_ADDR_START,
                );
            });
    }

    ArchInterface::prepare_drivers();

    if let Ok(fdt) = Fdt::new(&dt_buf) {
        for node in fdt.all_nodes() {
            ArchInterface::try_to_add_device(&node);
        }
    }

    drop(dt_buf);

    // enable fp
    CPACR_EL1.write(CPACR_EL1::FPEN::TrapNothing);
    barrier::isb(barrier::SY);

    ArchInterface::main(hart_id);

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
