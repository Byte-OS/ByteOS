use crate::{PAGE_ITEM_COUNT, PTE, PTEFlags};

/// 汇编入口函数
///
/// 分配栈 初始化页表信息 并调到rust入口函数
#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _start() -> ! {
    const STACK_SIZE: usize = 4096 * 4 * 8;

    #[link_section = ".bss.stack"]
    static mut STACK: [u8; STACK_SIZE] = [0u8; STACK_SIZE];

    #[link_section = ".data.prepage"]
    static mut PAGE_TABLE: [PTE; PAGE_ITEM_COUNT] = {
        let mut arr: [PTE; PAGE_ITEM_COUNT] = [PTE::new(); PAGE_ITEM_COUNT];
        // 初始化页表信息
        // 0x00000000_00000000 -> 0x00000000 (1G)
        // 0x00000000_80000000 -> 0x80000000 (1G)
        // 0xffffffff_c0000000 -> 0x80000000 (1G)
        // 0xffffffff_40000000 -> 0x00000000 (1G)
        // arr[0] = PTE::from_addr(0x0).set_flags(PTEFlags::VRWX);
        arr[2] = PTE::from_addr(0x8000_0000, PTEFlags::VRWX);
        arr[509] = PTE::from_addr(0x0, PTEFlags::GVRWX);
        arr[511] = PTE::from_addr(0x8000_0000, PTEFlags::GVRWX);
        arr
    };

    core::arch::asm!(
        // 1. 设置栈信息
        // sp = bootstack + (hartid + 1) * 0x10000
        "   add     t0, a0, 1
            slli    t0, t0, 14
            lui     sp, %hi({stack})
            add     sp, sp, t0 ",
        // 2. 开启分页模式
        // satp = (8 << 60) | PPN(page_table)
        "   lui     t0, %hi({page_table})
            li      t1, 0xffffffffc0000000 - 0x80000000
            sub     t0, t0, t1
            srli    t0, t0, 12
            li      t1, 8 << 60
            or      t0, t0, t1
            csrw    satp, t0
            sfence.vma",
        // 3. 跳到 rust_main 函数，绝对路径
        "   lui     t0, %hi(rust_main)
            addi    t0, t0, %lo(rust_main)
            jr      t0
        ",
        stack      = sym STACK,
        page_table = sym PAGE_TABLE,
        options(noreturn),
    )
}
