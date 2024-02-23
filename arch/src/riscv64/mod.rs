mod boards;
mod consts;
mod context;
mod entry;
mod interrupt;
mod page_table;
mod sbi;
mod timer;

use alloc::vec::Vec;
pub use boards::*;
pub use consts::*;
pub use context::*;
pub use entry::switch_to_kernel_page_table;
use fdt::Fdt;
pub use interrupt::{
    enable_external_irq, enable_irq, init_interrupt, trap_pre_handle, user_restore,
};
pub use page_table::*;
pub use sbi::*;
pub use timer::*;

use riscv::register::sstatus;

use crate::ArchInterface;

#[no_mangle]
extern "C" fn rust_main(hartid: usize, device_tree: usize) {
    crate::clear_bss();
    ArchInterface::init_logging();
    // Init allocator
    allocator::init();

    let (hartid, device_tree) = boards::init_device(hartid, device_tree);

    let mut dt_buf = Vec::new();

    if device_tree != 0 {
        let fdt = unsafe { Fdt::from_ptr(device_tree as *const u8).unwrap() };

        dt_buf.extend_from_slice(unsafe {
            core::slice::from_raw_parts(device_tree as *const u8, fdt.total_size())
        });

        info!("There has {} CPU(s)", fdt.cpus().count());

        fdt.memory().regions().for_each(|x| {
            info!(
                "memory region {:#X} - {:#X}",
                x.starting_address as usize,
                x.starting_address as usize + x.size.unwrap()
            );

            ArchInterface::add_memory_region(
                x.starting_address as usize | VIRT_ADDR_START,
                (x.starting_address as usize + x.size.unwrap()) | VIRT_ADDR_START
            );
        });
    }

    ArchInterface::prepare_drivers();

    if let Ok(fdt) = Fdt::new(&dt_buf) {
        for node in fdt.all_nodes() {
            ArchInterface::try_to_add_device(&node);
        }
    }

    // 开启 SUM
    unsafe {
        // 开启浮点运算
        sstatus::set_fs(sstatus::FS::Dirty);
    }

    crate::ArchInterface::main(hartid);
    shutdown();
}

#[inline]
pub fn wfi() {
    unsafe {
        riscv::register::sstatus::clear_sie();
        riscv::asm::wfi();
        riscv::register::sstatus::set_sie();
    }
}
