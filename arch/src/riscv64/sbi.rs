//! 调用 Machine 层的操作
// 目前还不会用到全部的 SBI 调用，暂时允许未使用的变量或函数
#![allow(unused)]

use core::arch::asm;

use sbi_rt::{NoReason, Shutdown};

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
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!("ecall",
        in("a7") which,
        inlateout("a0") arg0 => ret,
        in("a1") arg1,
        in("a2") arg2);
    }
    ret
}

/// 设置定时器
#[inline]
pub fn set_timer(time: usize) {
    sbi_rt::set_timer(time as _);
}

/// 输出一个字符到屏幕
#[inline]
#[allow(deprecated)]
pub fn console_putchar(ch: u8) {
    sbi_rt::legacy::console_putchar(ch as _);
}

/// 获取输入
#[inline]
#[allow(deprecated)]
pub fn console_getchar() -> Option<u8> {
    let c = sbi_rt::legacy::console_getchar() as u8;
    match c == u8::MAX {
        true => None,
        _ => Some(c),
    }
}

/// 调用 SBI_SHUTDOWN 来关闭操作系统（直接退出 QEMU）
#[inline]
pub fn shutdown() -> ! {
    // sbi_rt::legacy::shutdown();
    sbi_rt::system_reset(Shutdown, NoReason);
    unreachable!()
}
