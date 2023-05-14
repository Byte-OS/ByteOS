use crate::VIRT_ADDR_START;
use crate::{PTEFlags, PAGE_ITEM_COUNT, PTE};

/// 汇编入口函数
///
/// 分配栈 初始化页表信息 并调到rust入口函数
#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 0x8000;

    #[link_section = ".bss.stack"]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    #[link_section = ".data.prepage"]
    static mut PAGE_TABLE: [PTE; PAGE_ITEM_COUNT] = {
        let mut arr: [PTE; PAGE_ITEM_COUNT] = [PTE::new(); PAGE_ITEM_COUNT];
        // 初始化页表信息
        // 0x00000000_80000000 -> 0x80000000 (1G)
        // 高半核
        // 0xffffffc0_00000000 -> 0x00000000 (1G)
        // 0xffffffc0_80000000 -> 0x80000000 (1G)
        arr[0] = PTE::from_addr(0x0000_0000, PTEFlags::VRWX);
        arr[1] = PTE::from_addr(0x4000_0000, PTEFlags::VRWX);
        arr[2] = PTE::from_addr(0x8000_0000, PTEFlags::VRWX);
        arr[0x100] = PTE::from_addr(0x0000_0000, PTEFlags::GVRWX);
        arr[0x101] = PTE::from_addr(0x4000_0000, PTEFlags::GVRWX);
        arr[0x102] = PTE::from_addr(0x8000_0000, PTEFlags::GVRWX);
        arr
    };

    core::arch::asm!(
        // 1. 设置栈信息
        // sp = bootstack + (hartid + 1) * 0x10000
        "
            la      sp, {boot_stack}
            li      t0, {stack_size}
            add     sp, sp, t0              // set boot stack

            li      s0, {virt_addr_start}   // add virtual address
            add     sp, sp, s0
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
            la      a2, rust_main
            add     a2, a2, s0
            jalr    a2                      // call rust_main
        ",
        stack_size = const STACK_SIZE,
        boot_stack = sym STACK,
        page_table = sym PAGE_TABLE,
        virt_addr_start = const VIRT_ADDR_START,
        options(noreturn),
    )
}
