mod addr;
mod boards;
mod consts;
mod context;
mod entry;
mod interrupt;
mod page_table;
mod sbi;
mod timer;

pub use addr::*;
pub use boards::*;
pub use consts::*;
pub use context::*;
pub use entry::switch_to_kernel_page_table;
pub use interrupt::{
    enable_external_irq, enable_irq, init_interrupt, trap_pre_handle, user_restore,
};
pub use page_table::*;
pub use sbi::*;
pub use timer::*;

use riscv::register::sstatus;

#[no_mangle]
extern "C" fn rust_main(hartid: usize, device_tree: usize) {
    crate::clear_bss();
    crate::prepare_init();

    let (hartid, device_tree) = boards::init_device(hartid, device_tree);

    // 开启 SUM
    unsafe {
        // 开启浮点运算
        sstatus::set_fs(sstatus::FS::Dirty);
    }
    crate::ArchInterface::main(hartid, device_tree);
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
