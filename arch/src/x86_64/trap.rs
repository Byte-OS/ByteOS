use core::arch::global_asm;

use x86::{controlregs::cr2, irq::*};

use crate::Context;

global_asm!(include_str!("trap.S"));

#[percpu::def_percpu]
static KERNEL_RSP: usize = 1;

#[no_mangle]
fn x86_trap_handler(tf: &mut Context) {
    match tf.vector as u8 {
        PAGE_FAULT_VECTOR => {
            panic!(
                "Kernel #PF @ {:#x}, fault_vaddr={:#x}, error_code={:#x}:\n{:#x?}",
                tf.rip,
                unsafe { cr2() },
                tf.error_code,
                tf,
            );
        }
        BREAKPOINT_VECTOR => debug!("#BP @ {:#x} ", tf.rip),
        GENERAL_PROTECTION_FAULT_VECTOR => {
            panic!(
                "#GP @ {:#x}, error_code={:#x}:\n{:#x?}",
                tf.rip, tf.error_code, tf
            );
        }
        // IRQ_VECTOR_START..=IRQ_VECTOR_END => crate::trap::handle_irq_extern(tf.vector as _),
        _ => {
            panic!(
                "Unhandled exception {} (error_code = {:#x}) @ {:#x}:\n{:#x?}",
                tf.vector, tf.error_code, tf.rip, tf
            );
        }
    }
}
