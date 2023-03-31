//! 调用 Machine 层的操作
// 目前还不会用到全部的 SBI 调用，暂时允许未使用的变量或函数
#![allow(unused)]

use core::arch::asm;

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUT_CHAR: usize = 1;
const SBI_CONSOLE_GET_CHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
const SBI_SHUTDOWN: usize = 8;

// SBI 调用
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> i32 {
    let mut ret;
    unsafe {
        asm!("ecall",
        in("a7") which,
        inlateout("a0") arg0 as i32 => ret,
        in("a1") arg1,
        in("a2") arg2);
    }
    ret
}

/// 设置定时器
pub fn set_timer(time: usize) {
    sbi_call(SBI_SET_TIMER, time, 0, 0);
}

/// 输出一个字符到屏幕
pub fn console_putchar(ch: u8) {
    sbi_call(SBI_CONSOLE_PUT_CHAR, ch as usize, 0, 0);
}

/// 获取输入
pub fn console_getchar() -> char {
    sbi_call(SBI_CONSOLE_GET_CHAR, 0, 0, 0) as u8 as char
}

/// 调用 SBI_SHUTDOWN 来关闭操作系统（直接退出 QEMU）
pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    unreachable!()
}

#[repr(C)]
pub struct SbiRet {
    /// Error number
    pub error: usize,
    /// Result value
    pub value: usize,
}

pub const EXTENSION_BASE: usize = 0x10;
pub const EXTENSION_TIMER: usize = 0x54494D45;
pub const EXTENSION_IPI: usize = 0x735049;
pub const EXTENSION_RFENCE: usize = 0x52464E43;
pub const EXTENSION_HSM: usize = 0x48534D;
pub const EXTENSION_SRST: usize = 0x53525354;

const FUNCTION_HSM_HART_START: usize = 0x0;
const FUNCTION_HSM_HART_STOP: usize = 0x1;
const FUNCTION_HSM_HART_GET_STATUS: usize = 0x2;
const FUNCTION_HSM_HART_SUSPEND: usize = 0x3;

#[inline(always)]
fn sbi_call_3(extension: usize, function: usize, arg0: usize, arg1: usize, arg2: usize) -> SbiRet {
    let (error, value);
    unsafe {
        asm!(
            "ecall",
            in("a0") arg0, in("a1") arg1, in("a2") arg2,
            in("a6") function, in("a7") extension,
            lateout("a0") error, lateout("a1") value,
        )
    }
    SbiRet { error, value }
}

pub fn hart_suspend(suspend_type: u32, resume_addr: usize, opaque: usize) -> SbiRet {
    sbi_call_3(
        EXTENSION_HSM,
        FUNCTION_HSM_HART_SUSPEND,
        suspend_type as usize,
        resume_addr,
        opaque,
    )
}
