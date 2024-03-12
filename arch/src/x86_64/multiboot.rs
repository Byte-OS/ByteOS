extern crate core;

use crate::x86_64::rust_tmp_main;
use crate::{BOOT_STACK, STACK_SIZE};
use core::arch::global_asm;
use core::{mem, slice};
use multiboot::information::{MemoryManagement, Multiboot, PAddr};
use x86_64::registers::control::{Cr0Flags, Cr4Flags};
use x86_64::registers::model_specific::EferFlags;

use crate::VIRT_ADDR_START;

/// Flags set in the 'flags' member of the multiboot header.
///
/// (bits 1, 16: memory information, address fields in header)
const MULTIBOOT_HEADER_FLAGS: usize = 0x0001_0002;

/// The magic field should contain this.
const MULTIBOOT_HEADER_MAGIC: usize = 0x1BADB002;

const CR0: u64 = Cr0Flags::PROTECTED_MODE_ENABLE.bits()
    | Cr0Flags::MONITOR_COPROCESSOR.bits()
    | Cr0Flags::NUMERIC_ERROR.bits()
    | Cr0Flags::WRITE_PROTECT.bits()
    | Cr0Flags::PAGING.bits();

const CR4: u64 = Cr4Flags::PHYSICAL_ADDRESS_EXTENSION.bits()
    | Cr4Flags::PAGE_GLOBAL.bits()
    | Cr4Flags::OSFXSR.bits()
    | Cr4Flags::OSXMMEXCPT_ENABLE.bits();
const EFER: u64 = EferFlags::LONG_MODE_ENABLE.bits() | EferFlags::NO_EXECUTE_ENABLE.bits();

static mut MEM: Mem = Mem;

struct Mem;

impl MemoryManagement for Mem {
    unsafe fn paddr_to_slice(&self, addr: PAddr, size: usize) -> Option<&'static [u8]> {
        let ptr = mem::transmute(addr | VIRT_ADDR_START as u64);
        Some(slice::from_raw_parts(ptr, size))
    }

    // If you only want to read fields, you can simply return `None`.
    unsafe fn allocate(&mut self, _length: usize) -> Option<(PAddr, &mut [u8])> {
        None
    }

    unsafe fn deallocate(&mut self, addr: PAddr) {
        if addr != 0 {
            unimplemented!()
        }
    }
}

/// mboot_ptr is the initial pointer to the multiboot structure
/// provided in %ebx on start-up.
pub fn use_multiboot(mboot_ptr: PAddr) -> Option<Multiboot<'static, 'static>> {
    unsafe { Multiboot::from_ptr(mboot_ptr, &mut MEM) }
}

global_asm!(
    include_str!("multiboot.S"),
    mb_hdr_magic = const MULTIBOOT_HEADER_MAGIC,
    mb_hdr_flags = const MULTIBOOT_HEADER_FLAGS,
    entry = sym rust_tmp_main,

    offset = const VIRT_ADDR_START,
    boot_stack_size = const STACK_SIZE,
    boot_stack = sym BOOT_STACK,

    cr0 = const CR0,
    cr4 = const CR4,
    efer_msr = const x86::msr::IA32_EFER,
    efer = const EFER,
);

#[no_mangle]
pub fn switch_to_kernel_page_table() {
    unsafe {
        core::arch::asm!(
            "
                lea     rax, [kernel_page_table - {offset}]
                mov     cr3, rax
            ", 
            offset = const VIRT_ADDR_START
        );
    }
}
