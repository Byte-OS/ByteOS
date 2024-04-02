mod boards;
mod consts;
mod context;
mod entry;
mod interrupt;
#[cfg(feature = "kcontext")]
mod kcontext;
mod page_table;
mod sbi;
mod timer;

use alloc::vec::Vec;
pub use boards::*;
pub use consts::*;
pub use context::TrapFrame;
pub use entry::{kernel_page_table, switch_to_kernel_page_table};
use fdt::Fdt;
pub use interrupt::{
    disable_irq, enable_external_irq, enable_irq, init_interrupt, run_user_task,
    run_user_task_forever,
};
pub use page_table::*;
pub use sbi::*;
pub use timer::*;

#[cfg(feature = "kcontext")]
pub use kcontext::{context_switch, context_switch_pt, read_current_tp, KContext};

use riscv::register::sstatus;

use crate::{pagetable::MappingFlags, ArchInterface, VirtPage};

use self::entry::secondary_start;

#[percpu::def_percpu]
static CPU_ID: usize = 0;

pub(crate) fn rust_main(hartid: usize, device_tree: usize) {
    crate::clear_bss();
    // Init allocator
    percpu::init(4);
    percpu::set_local_thread_pointer(hartid);
    CPU_ID.write_current(hartid);

    ArchInterface::init_allocator();

    ArchInterface::init_logging();

    interrupt::init_interrupt();

    let mut cpu_num = 0;
    let (hartid, device_tree) = boards::init_device(hartid, device_tree);

    let mut dt_buf = Vec::new();
    if device_tree != 0 {
        let fdt = unsafe { Fdt::from_ptr(device_tree as *const u8).unwrap() };

        dt_buf.extend_from_slice(unsafe {
            core::slice::from_raw_parts(device_tree as *const u8, fdt.total_size())
        });

        cpu_num = fdt.cpus().count();

        info!("There has {} CPU(s)", fdt.cpus().count());

        fdt.memory().regions().for_each(|x| {
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

    // 开启 SUM
    unsafe {
        // 开启浮点运算
        sstatus::set_fs(sstatus::FS::Dirty);
    }

    drop(dt_buf);

    let page_table = PageTable::current();

    (0..cpu_num).into_iter().for_each(|cpu| {
        if cpu == CPU_ID.read_current() {
            return;
        };

        // PERCPU DATA ADDRESS RANGE END
        let cpu_addr_end = MULTI_CORE_AREA + (cpu + 1) * MULTI_CORE_AREA_SIZE;
        let aux_core_func = (secondary_start as usize) & (!VIRT_ADDR_START);

        // Ready to build multi core area.
        // default stack size is 512K
        for i in 0..128 {
            page_table.map(
                ArchInterface::frame_alloc_persist(),
                VirtPage::from_addr(cpu_addr_end - i * PAGE_SIZE - 1),
                MappingFlags::RWX | MappingFlags::G,
                3,
            )
        }

        info!("secondary addr: {:#x}", secondary_start as usize);
        let ret = sbi_rt::hart_start(cpu, aux_core_func, cpu_addr_end);
        if ret.is_ok() {
            info!("hart {} Startting successfully", cpu);
        } else {
            warn!("hart {} Startting failed", cpu)
        }
    });

    crate::ArchInterface::main(hartid);
    shutdown();
}

pub(crate) extern "C" fn rust_secondary_main(hartid: usize) {
    percpu::set_local_thread_pointer(hartid);
    CPU_ID.write_current(hartid);

    info!("secondary hart {} started", hartid);
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

pub fn hart_id() -> usize {
    CPU_ID.read_current()
}
