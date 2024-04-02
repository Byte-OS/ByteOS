use core::arch::riscv64::sfence_vma_all;

use crate::{PTEFlags, PAGE_ITEM_COUNT, PTE};
use crate::{PageTable, VIRT_ADDR_START};

#[link_section = ".data.prepage.entry"]
pub(crate) static mut PAGE_TABLE: [PTE; PAGE_ITEM_COUNT] = {
    let mut arr: [PTE; PAGE_ITEM_COUNT] = [PTE::new(); PAGE_ITEM_COUNT];
    // 初始化页表信息
    // 0x00000000_80000000 -> 0x80000000 (1G)
    // 高半核
    // 0xffffffc0_00000000 -> 0x00000000 (1G)
    // 0xffffffc0_80000000 -> 0x80000000 (1G)

    // arr[0] = PTE::from_addr(0x0000_0000, PTEFlags::VRWX);
    // arr[1] = PTE::from_addr(0x4000_0000, PTEFlags::VRWX);
    arr[2] = PTE::from_addr(0x8000_0000, PTEFlags::ADVRWX);
    arr[0x100] = PTE::from_addr(0x0000_0000, PTEFlags::ADGVRWX);
    arr[0x101] = PTE::from_addr(0x4000_0000, PTEFlags::ADGVRWX);
    arr[0x102] = PTE::from_addr(0x8000_0000, PTEFlags::ADGVRWX);
    arr[0x106] = PTE::from_addr(0x8000_0000, PTEFlags::ADVRWX);
    arr
};

/// 汇编入口函数
///
/// 分配栈 初始化页表信息 并调到rust入口函数
#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        // 1. 设置栈信息
        // sp = bootstack + (hartid + 1) * 0x10000
        "
            la      sp, {boot_stack}
            li      t0, {stack_size}
            add     sp, sp, t0              // set boot stack

            li      s0, {virt_addr_start}   // add virtual address
            or      sp, sp, s0
        ",
        // 2. 开启分页模式
        // satp = (8 << 60) | PPN(page_table)
        "
            la      t0, {page_table}
            srli    t0, t0, 12
            li      t1, 8 << 60
            or      t0, t0, t1
            csrw    satp, t0
            sfence.vma
        ",
        // 3. 跳到 rust_main 函数，绝对路径
        "
            la      a2, {entry}
            or      a2, a2, s0
            jalr    a2                      // call rust_main
        ",
        stack_size = const crate::STACK_SIZE,
        boot_stack = sym crate::BOOT_STACK,
        page_table = sym PAGE_TABLE,
        entry = sym super::rust_main,
        virt_addr_start = const VIRT_ADDR_START,
        options(noreturn),
    )
}

/// 汇编函数入口
///
/// 初始化也表信息 并调到 rust_secondary_main 入口函数

#[naked]
#[no_mangle]
pub(crate) unsafe extern "C" fn secondary_start() -> ! {
    core::arch::asm!(
        // 1. 设置栈信息
        // sp = bootstack + (hartid + 1) * 0x10000
        "
            mv      s6, a0
            mv      sp, a1

            li      s0, {virt_addr_start}   // add virtual address
            or      sp, sp, s0
        ",
        // 2. 开启分页模式
        // satp = (8 << 60) | PPN(page_table)
        "
            la      t0, {page_table}
            srli    t0, t0, 12
            li      t1, 8 << 60
            or      t0, t0, t1
            csrw    satp, t0
            sfence.vma
        ", 
        // 3. 跳到 secondary_entry
        "
            la      a2, {entry}
            or      a2, a2, s0
            mv      a0, s6
            jalr    a2                      // call rust_main
        ",
        page_table = sym PAGE_TABLE,
        entry = sym super::rust_secondary_main,
        virt_addr_start = const VIRT_ADDR_START,
        options(noreturn)
    );
}

pub fn switch_to_kernel_page_table() {
    unsafe {
        riscv::register::satp::set(
            riscv::register::satp::Mode::Sv39,
            0,
            (PAGE_TABLE.as_ptr() as usize & !VIRT_ADDR_START) >> 12,
        );
        sfence_vma_all();
    }
}

pub fn kernel_page_table() -> PageTable {
    PageTable(crate::PhysAddr(unsafe {
        PAGE_TABLE.as_ptr() as usize & !VIRT_ADDR_START
    }))
}
