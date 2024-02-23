use x86::bits64::paging::{PDEntry, PDFlags, PTEntry, PTFlags, PAGE_SIZE_ENTRIES, PD, PT};

use crate::VIRT_ADDR_START;

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
            mov  rax, 15
            syscall
        ",
        options(noreturn)
    )
}

#[link_section = ".data.prepage"]
static mut TRX_STEP1: PD = [PDEntry(0); PAGE_SIZE_ENTRIES];

#[link_section = ".data.prepage"]
static mut TRX_STEP2: PT = [PTEntry(0); PAGE_SIZE_ENTRIES];

pub fn init() {
    unsafe {
        TRX_STEP1[0] = PDEntry::new((TRX_STEP2.as_ptr() as usize & !VIRT_ADDR_START).into(), PDFlags::P | PDFlags::RW);
        TRX_STEP2[0] = PTEntry::new((_sigreturn as usize & !VIRT_ADDR_START).into(), PTFlags::P | PTFlags::US);
    }
}
