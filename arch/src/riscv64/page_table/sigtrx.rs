use crate::{PTEFlags, PAGE_ITEM_COUNT, PTE, VIRT_ADDR_START};

/// 汇编入口函数
///
/// 分配栈 初始化页表信息 并调到rust入口函数
#[naked]
#[no_mangle]
#[link_section = ".sigtrx.sigreturn"]
unsafe extern "C" fn _sigreturn() -> ! {
    core::arch::asm!(
        // 1. 设置栈信息
        // sp = bootstack + (hartid + 1) * 0x10000
        "
            li  a7, 139
            ecall
        ",
        options(noreturn)
    )
}

#[link_section = ".data.prepage.trx1"]
static mut TRX_STEP1: [PTE; PAGE_ITEM_COUNT] = [PTE::new(); PAGE_ITEM_COUNT];

#[link_section = ".data.prepage.trx2"]
static mut TRX_STEP2: [PTE; PAGE_ITEM_COUNT] = [PTE::new(); PAGE_ITEM_COUNT];

pub fn init() {
    unsafe {
        TRX_STEP1[0] = PTE::from_addr(_sigreturn as usize & !VIRT_ADDR_START, PTEFlags::UVRX);
        TRX_STEP2[0] = PTE::from_addr(TRX_STEP1.as_ptr() as usize & !VIRT_ADDR_START, PTEFlags::V);
    }
}

pub fn get_trx_mapping() -> usize {
    unsafe { TRX_STEP2.as_ptr() as usize & !VIRT_ADDR_START }
}
