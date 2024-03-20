use core::arch::{asm, global_asm};

use aarch64_cpu::registers::{Writeable, ESR_EL1, FAR_EL1, VBAR_EL1};
use tock_registers::interfaces::Readable;

use crate::{
    aarch64::{gic::handle_irq, timer::set_next_timer},
    ArchInterface, TrapType,
};

use super::Context;

global_asm!(include_str!("trap.S"));

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
fn handle_exception(tf: &mut Context, kind: TrapKind, source: TrapSource) -> TrapType {
    if kind == TrapKind::Irq {
        set_next_timer();
        handle_irq(|_irq| {});
        return TrapType::Time;
    }
    if kind != TrapKind::Synchronous {
        panic!(
            "Invalid exception {:?} from {:?}:\n{:#x?}",
            kind, source, tf
        );
    }
    let esr = ESR_EL1.extract();
    let trap_type = match esr.read_as_enum(ESR_EL1::EC) {
        Some(ESR_EL1::EC::Value::Brk64) => {
            let iss = esr.read(ESR_EL1::ISS);
            debug!("BRK #{:#x} @ {:#x} ", iss, tf.elr);
            tf.elr += 4;
            TrapType::Breakpoint
        }
        Some(ESR_EL1::EC::Value::SVC64) => TrapType::UserEnvCall,
        Some(ESR_EL1::EC::Value::DataAbortLowerEL)
        | Some(ESR_EL1::EC::Value::InstrAbortLowerEL) => {
            let iss = esr.read(ESR_EL1::ISS);
            warn!(
                "EL0 Page Fault @ {:#x}, FAR={:#x}, ISS={:#x}",
                tf.elr,
                FAR_EL1.get(),
                iss
            );
            TrapType::InstructionPageFault(FAR_EL1.get() as _)
        }
        Some(ESR_EL1::EC::Value::DataAbortCurrentEL)
        | Some(ESR_EL1::EC::Value::InstrAbortCurrentEL) => {
            let iss = esr.read(ESR_EL1::ISS);
            warn!(
                "EL1 Page Fault @ {:#x}, FAR={:#x}, ISS={:#x}:\n{:#x?}",
                tf.elr,
                FAR_EL1.get(),
                iss,
                tf,
            );
            TrapType::InstructionPageFault(FAR_EL1.get() as _)
        }
        _ => {
            panic!(
                "Unhandled synchronous exception @ {:#x}: ESR={:#x} (EC {:#08b}, ISS {:#x})",
                tf.elr,
                esr.get(),
                esr.read(ESR_EL1::EC),
                esr.read(ESR_EL1::ISS),
            );
        }
    };
    ArchInterface::kernel_interrupt(tf, trap_type);
    trap_type
}

pub fn init() {
    extern "C" {
        fn exception_vector_base();
    }
    VBAR_EL1.set(exception_vector_base as _);
}

// 设置中断
pub fn init_interrupt() {
    // unsafe {
    //     asm!("brk #0");
    // }
    // enable_irq();
}

#[naked]
extern "C" fn user_restore(context: *mut Context) -> TrapKind {
    unsafe {
        asm!(
            r"
            sub     sp, sp, 18 * 8
            stp     x8, x16, [sp]
            stp     x17, x18, [sp, 2 * 8]
            stp     x19, x20, [sp, 4 * 8]
            stp     x21, x22, [sp, 6 * 8]
            stp     x23, x24, [sp, 8 * 8]
            stp     x25, x26, [sp, 10 * 8]
            stp     x27, x28, [sp, 12 * 8]
            stp     x29, x30, [sp, 14 * 8]
            str     x0, [sp, 16 * 8]

            ldr     x12, [x0, 34 * 8]
            ldp     x10, x11, [x0, 32 * 8]
            ldp     x30, x9, [x0, 30 * 8]
            msr     sp_el0, x9
            msr     elr_el1, x10
            msr     spsr_el1, x11
            msr     tpidr_el0, x12

            ldp     x28, x29, [x0, 28 * 8]
            ldp     x26, x27, [x0, 26 * 8]
            ldp     x24, x25, [x0, 24 * 8]
            ldp     x22, x23, [x0, 22 * 8]
            ldp     x20, x21, [x0, 20 * 8]
            ldp     x18, x19, [x0, 18 * 8]
            ldp     x16, x17, [x0, 16 * 8]
            ldp     x14, x15, [x0, 14 * 8]
            ldp     x12, x13, [x0, 12 * 8]
            ldp     x10, x11, [x0, 10 * 8]
            ldp     x8, x9,   [x0, 8 * 8]
            ldp     x6, x7,   [x0, 6 * 8]
            ldp     x4, x5,   [x0, 4 * 8]
            ldp     x2, x3,   [x0, 2 * 8]
            ldp     x0, x1,   [x0]
            eret
        ",
            options(noreturn)
        )
    }
}

pub fn run_user_task(cx: &mut Context) -> Option<()> {
    let trap_kind = user_restore(cx);
    match handle_exception(cx, trap_kind, TrapSource::LowerAArch64) {
        TrapType::UserEnvCall => Some(()),
        _ => None,
    }
}

#[allow(dead_code)]
#[inline(always)]
pub fn enable_irq() {
    unsafe { asm!("msr daifclr, #2") };
}

#[inline(always)]
pub fn enable_external_irq() {
    // unsafe {
    //     sie::set_sext();
    // }
}
