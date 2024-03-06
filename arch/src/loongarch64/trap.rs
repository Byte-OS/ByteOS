use core::arch::{asm, global_asm};

use crate::TrapType;

use super::Context;

#[repr(u8)]
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum TrapKind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
enum TrapSource {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[no_mangle]
fn handle_exception(tf: &mut Context, kind: TrapKind, source: TrapSource) {
    todo!("handle_exception")
}

pub fn init() {
    todo!("init interrupt")
}

// 设置中断
pub fn init_interrupt() {
    todo!("test brk");
    enable_irq();
}

pub fn trap_pre_handle(tf: &mut Context) -> TrapType {
    todo!("trap_pre_handle")
}

#[naked]
#[no_mangle]
pub extern "C" fn user_restore(context: *mut Context) {
    unsafe {
        asm!(
            r"
            
        ",
            options(noreturn)
        )
    }
}

#[allow(dead_code)]
#[inline(always)]
pub fn enable_irq() {
    todo!("enable_irq")
}

#[inline(always)]
pub fn enable_external_irq() {
    // unsafe {
    //     sie::set_sext();
    // }
}
