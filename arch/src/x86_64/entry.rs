use crate::VIRT_ADDR_START;
use crate::{PTEFlags, PAGE_ITEM_COUNT, PTE};

const STACK_SIZE: usize = 0x80000;

#[link_section = ".bss.stack"]
static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

#[link_section = ".data.prepage.entry"]
static mut PAGE_TABLE: [PTE; PAGE_ITEM_COUNT] = {
    let mut arr: [PTE; PAGE_ITEM_COUNT] = [PTE::new(); PAGE_ITEM_COUNT];
    // 初始化页表信息
    // 0x00000000_80000000 -> 0x80000000 (1G)
    // 高半核
    // 0xffffffc0_00000000 -> 0x00000000 (1G)
    // 0xffffffc0_80000000 -> 0x80000000 (1G)

    // arr[0] = PTE::from_addr(0x0000_0000, PTEFlags::VRWX);
    // arr[1] = PTE::from_addr(0x4000_0000, PTEFlags::VRWX);
    arr[2] = PTE::from_addr(
        0x8000_0000,
        PTEFlags::VRWX.union(PTEFlags::D).union(PTEFlags::A),
    );
    arr[0x100] = PTE::from_addr(0x0000_0000, PTEFlags::ADGVRWX);
    arr[0x101] = PTE::from_addr(0x4000_0000, PTEFlags::ADGVRWX);
    arr[0x102] = PTE::from_addr(0x8000_0000, PTEFlags::ADGVRWX);
    arr[0x106] = PTE::from_addr(0x8000_0000, PTEFlags::ADVRWX);
    arr
};

// /// 汇编入口函数
// ///
// /// 分配栈 初始化页表信息 并调到rust入口函数
// #[naked]
// #[no_mangle]
// #[link_section = ".text.entry"]
// unsafe extern "C" fn _start() -> ! {
//     core::arch::asm!(
//         // // 1. 设置栈信息
//         // // sp = bootstack + (hartid + 1) * 0x10000
//         // "
//         //     la      sp, {boot_stack}
//         //     li      t0, {stack_size}
//         //     add     sp, sp, t0              // set boot stack

//         //     li      s0, {virt_addr_start}   // add virtual address
//         //     or      sp, sp, s0
//         // ",
//         // // 2. 开启分页模式
//         // // satp = (8 << 60) | PPN(page_table)
//         // "
//         //     la      t0, {page_table}
//         //     srli    t0, t0, 12
//         //     li      t1, 8 << 60
//         //     or      t0, t0, t1
//         //     csrw    satp, t0
//         //     sfence.vma
//         // ",
//         // // 3. 跳到 rust_main 函数，绝对路径
//         // "
//         //     la      a2, rust_main
//         //     or      a2, a2, s0
//         //     jalr    a2                      // call rust_main
//         // ",
//         // stack_size = const STACK_SIZE,
//         // boot_stack = sym STACK,
//         // page_table = sym PAGE_TABLE,
//         // virt_addr_start = const VIRT_ADDR_START,
//         "",
//         options(noreturn),
//     )
// }

use core::arch::global_asm;

use x86_64::registers::control::{Cr0Flags, Cr4Flags};
use x86_64::registers::model_specific::EferFlags;

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
pub const PHYS_VIRT_OFFSET: usize = 0xffff_ff80_0000_0000;

global_asm!(
    include_str!("multiboot.S"),
    mb_magic = const MULTIBOOT_BOOTLOADER_MAGIC,
    mb_hdr_magic = const MULTIBOOT_HEADER_MAGIC,
    mb_hdr_flags = const MULTIBOOT_HEADER_FLAGS,
    entry = sym rust_tmp_main,
    entry_secondary = sym rust_entry_secondary,

    offset = const PHYS_VIRT_OFFSET,
    boot_stack_size = const STACK_SIZE,
    boot_stack = sym BOOT_STACK,

    cr0 = const CR0,
    cr4 = const CR4,
    efer_msr = const x86::msr::IA32_EFER,
    efer = const EFER,
);

fn rust_tmp_main() {

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
