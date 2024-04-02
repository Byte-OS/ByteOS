mod boot;
mod consts;
mod context;
mod gic;

#[cfg(feature = "kcontext")]
mod kcontext;
mod page_table;
mod pl011;
mod psci;
mod timer;
mod trap;

use aarch64_cpu::registers::{Writeable, TTBR0_EL1};
use aarch64_cpu::{asm::barrier, registers::CPACR_EL1};
use alloc::vec::Vec;
pub use consts::*;
pub use context::TrapFrame;
use fdt::Fdt;

#[cfg(feature = "kcontext")]
pub use kcontext::{context_switch, context_switch_pt, read_current_tp, KContext};

pub use page_table::*;
pub use pl011::{console_getchar, console_putchar};
pub use psci::system_off as shutdown;
pub use timer::{get_time, time_to_usec};
pub use trap::{enable_external_irq, enable_irq, init_interrupt, run_user_task};

use crate::{clear_bss, ArchInterface};

pub fn rust_tmp_main(hart_id: usize, device_tree: usize) {
    clear_bss();
    pl011::init_early();
    ArchInterface::init_logging();
    trap::init();
    ArchInterface::init_allocator();
    gic::init();

    timer::init();

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

    // Prepare the drivers. This operation just inject the driver to registry.
    ArchInterface::prepare_drivers();

    if let Ok(fdt) = Fdt::new(&dt_buf) {
        for node in fdt.all_nodes() {
            ArchInterface::try_to_add_device(&node);
        }
    }

    // Release the memory was allocated above.
    drop(dt_buf);

    // Enable Floating Point Feature.
    CPACR_EL1.write(CPACR_EL1::FPEN::TrapNothing);
    barrier::isb(barrier::SY);

    // Enter to kernel entry point(`main` function).
    ArchInterface::main(hart_id);

    shutdown();
}

pub fn kernel_page_table() -> PageTable {
    PageTable(crate::PhysAddr(TTBR0_EL1.get_baddr() as _))
}
