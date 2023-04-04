mod addr;
mod consts;
mod context;
mod entry;
mod interrupt;
mod page_table;
mod sbi;
mod timer;

pub use addr::*;
pub use consts::*;
pub use context::*;
pub use interrupt::init_interrupt;
pub use page_table::*;
pub use sbi::*;

use riscv::register::sstatus;

#[no_mangle]
extern "C" fn rust_main(hartid: usize, device_tree: usize) {
    extern "Rust" {
        fn main(hartid: usize, device_tree: usize);
    }

    // 内核中断初始化
    // interrupt::init();

    // 开启 SUM
    unsafe {
        // 开启浮点运算
        sstatus::set_fs(sstatus::FS::Dirty);

        // 开启SUM位 让内核可以访问用户空间  踩坑：
        // only in qemu. eg: qemu is riscv 1.10  NOTE: k210 is riscv 1.9.1
        // in 1.10 is SUM but in 1.9.1 is PUM which is the opposite meaning with SUM
        sstatus::set_sum();

        main(hartid, device_tree);
    }

    shutdown();
}
