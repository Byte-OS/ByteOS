use arch::{Context, TrapType};
use executor::get_current_task;
use log::warn;
use super::user::user_cow_int;

pub fn kernel_interrupt(_cx: &mut Context, trap_type: TrapType) {
    match trap_type {
        TrapType::StorePageFault(addr) | TrapType::InstructionPageFault(addr) => {
            // judge whether it is trigger by a user_task handler.
            if let Some(task) = get_current_task() {
                let cx_ref = unsafe { task.get_cx_ptr().as_mut() }.unwrap();
                user_cow_int(task, cx_ref, addr);
            } else {
                panic!("page fault: {:?}", trap_type);
            }
        },
        _ => {
            // warn!("trap_type: {:?}  context: {:#x?}", trap_type, cx);
            warn!("kernel_interrupt");
        }
    };
}