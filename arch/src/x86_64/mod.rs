mod apic;
mod consts;
mod context;
mod gdt;
mod idt;
mod interrupt;
mod multiboot;
mod page_table;
mod sigtrx;
mod time;
mod uart;

use ::multiboot::information::MemoryType;
pub use consts::*;
pub use context::Context;
pub use interrupt::*;
pub use multiboot::switch_to_kernel_page_table;
pub use page_table::*;
use raw_cpuid::CpuId;
pub use uart::*;

use x86::tlb;
use x86_64::{instructions::port::PortWriteOnly, registers::{control::{Cr4, Cr4Flags}, xcontrol::{XCr0, XCr0Flags}}};

use crate::{x86_64::multiboot::use_multiboot, ArchInterface, VirtAddr};

#[percpu::def_percpu]
static CPU_ID: usize = 1;

pub fn shutdown() -> ! {
    unsafe { PortWriteOnly::new(0x604).write(0x2000u16) };

    loop {}
}

fn rust_tmp_main(magic: usize, mboot_ptr: usize) {
    crate::clear_bss();
    idt::init();
    apic::init();
    sigtrx::init();
    ArchInterface::init_logging();
    // Init allocator
    allocator::init();
    percpu::init(1);
    percpu::set_local_thread_pointer(0);
    gdt::init();
    interrupt::init_syscall();
    time::init_early();
    
    // enable avx extend instruction set and sse if support avx
    // TIPS: QEMU not support avx, so we can't enable avx here
    // IF you want to use avx in the qemu, you can use -cpu IvyBridge-v2 to 
    // select a cpu with avx support
    CpuId::new().get_feature_info().map(|features| {
        info!("is there a avx feature: {}", features.has_avx());
        info!("is there a xsave feature: {}", features.has_xsave());
        info!("cr4 has OSXSAVE feature: {:?}", Cr4::read());
        if features.has_avx() && features.has_xsave() && Cr4::read().contains(Cr4Flags::OSXSAVE) {
            unsafe {
                XCr0::write(XCr0::read() | XCr0Flags::AVX | XCr0Flags::SSE | XCr0Flags::X87);
            }
        }
    });

    info!(
        "TEST CPU ID: {}  ptr: {:#x}",
        CPU_ID.read_current(),
        unsafe { CPU_ID.current_ptr() } as usize
    );
    CPU_ID.write_current(345);
    info!(
        "TEST CPU ID: {}  ptr: {:#x}",
        CPU_ID.read_current(),
        unsafe { CPU_ID.current_ptr() } as usize
    );

    info!("magic: {:#x}, mboot_ptr: {:#x}", magic, mboot_ptr);

    if let Some(mboot) = use_multiboot(mboot_ptr as _) {
        mboot
            .boot_loader_name()
            .inspect(|x| info!("bootloader: {}", x));
        mboot
            .command_line()
            .inspect(|x| info!("command_line: {}", x));
        if mboot.has_memory_map() {
            mboot
                .memory_regions()
                .unwrap()
                .filter(|x| x.memory_type() == MemoryType::Available)
                .for_each(|x| {
                    let start = x.base_address() as usize | VIRT_ADDR_START;
                    let end = x.length() as usize | VIRT_ADDR_START;
                    crate::ArchInterface::add_memory_region(start, end);
                });
        }
    }

    ArchInterface::prepare_drivers();

    crate::ArchInterface::main(0);

    shutdown()
}

#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    if let Some(vaddr) = vaddr {
        unsafe { tlb::flush(vaddr.into()) }
    } else {
        unsafe { tlb::flush_all() }
    }
}
