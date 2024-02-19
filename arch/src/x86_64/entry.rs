const STACK_SIZE: usize = 0x80000;

use core::arch::global_asm;
use crate::x86_64::idt;

use super::{pic, VIRT_ADDR_START};
use multiboot::information::MemoryType;
use x86_64::registers::control::{Cr0Flags, Cr4Flags};
use x86_64::registers::model_specific::EferFlags;

use super::multiboot::use_multiboot;

/// Flags set in the ’flags’ member of the multiboot header.
///
/// (bits 1, 16: memory information, address fields in header)
const MULTIBOOT_HEADER_FLAGS: usize = 0x0001_0002;

/// The magic field should contain this.
const MULTIBOOT_HEADER_MAGIC: usize = 0x1BADB002;

/// This should be in EAX.
pub(super) const MULTIBOOT_BOOTLOADER_MAGIC: usize = 0x2BADB002;

const CR0: u64 = Cr0Flags::PROTECTED_MODE_ENABLE.bits()
    | Cr0Flags::MONITOR_COPROCESSOR.bits()
    | Cr0Flags::NUMERIC_ERROR.bits()
    | Cr0Flags::WRITE_PROTECT.bits()
    | Cr0Flags::PAGING.bits();
const CR4: u64 = Cr4Flags::PHYSICAL_ADDRESS_EXTENSION.bits()
    | Cr4Flags::PAGE_GLOBAL.bits()
    | if cfg!(feature = "fp_simd") {
        Cr4Flags::OSFXSR.bits() | Cr4Flags::OSXMMEXCPT_ENABLE.bits()
    } else {
        0
    };
const EFER: u64 = EferFlags::LONG_MODE_ENABLE.bits() | EferFlags::NO_EXECUTE_ENABLE.bits();

#[link_section = ".bss.stack"]
static mut BOOT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

global_asm!(
    include_str!("multiboot.S"),
    mb_magic = const MULTIBOOT_BOOTLOADER_MAGIC,
    mb_hdr_magic = const MULTIBOOT_HEADER_MAGIC,
    mb_hdr_flags = const MULTIBOOT_HEADER_FLAGS,
    entry = sym rust_tmp_main,
    entry_secondary = sym rust_entry_secondary,

    offset = const VIRT_ADDR_START,
    boot_stack_size = const STACK_SIZE,
    boot_stack = sym BOOT_STACK,

    cr0 = const CR0,
    cr4 = const CR4,
    efer_msr = const x86::msr::IA32_EFER,
    efer = const EFER,
);

fn rust_tmp_main(magic: usize, mboot_ptr: usize) {
    crate::clear_bss();
    crate::prepare_init();
    super::idt::init();
    pic::init();

    info!("magic: {:#x}, mboot_ptr: {:#x}", magic, mboot_ptr);
    
    if let Some(mboot) = use_multiboot(mboot_ptr as _) {
        if mboot.has_memory_map() {
            info!("has memory map");
            mboot.memory_regions().unwrap().filter(|x| x.memory_type() == MemoryType::Available).for_each(|x| {
                let start = x.base_address() as usize | VIRT_ADDR_START;
                let end = x.length() as usize | VIRT_ADDR_START;
                info!("memory region: {:#x} length: {:#x}, type: {:#x?}", start, end, x.memory_type());
                crate::ArchInterface::add_memory_region(start, end);
            });
        }
    }
    
    crate::ArchInterface::main(0, 0);
}

fn rust_entry_secondary() {

}


pub fn switch_to_kernel_page_table() {
    // unsafe {
    //     riscv::register::satp::set(
    //         riscv::register::satp::Mode::Sv39,
    //         0,
    //         (PAGE_TABLE.as_ptr() as usize & !VIRT_ADDR_START) >> 12,
    //     );
    //     sfence_vma_all();
    // }
}
