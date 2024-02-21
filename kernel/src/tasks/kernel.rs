use crate::user::user_cow_int;
use arch::{Context, TrapType, VIRT_ADDR_START};
use devices::get_int_device;
use executor::get_current_task;

pub fn kernel_interrupt(_cx: &mut Context, trap_type: TrapType) {
    match trap_type {
        TrapType::StorePageFault(addr) | TrapType::InstructionPageFault(addr) => {
            if addr > VIRT_ADDR_START {
                panic!("kernel error: {:#x}", addr);
            }
            // judge whether it is trigger by a user_task handler.
            if let Some(task) = get_current_task() {
                let cx_ref = task.force_cx_ref();
                if task.pcb.is_locked() {
                    // task.pcb.force_unlock();
                    unsafe {
                        task.pcb.force_unlock();
                    }
                }
                user_cow_int(task, cx_ref, addr);
            } else {
                panic!("page fault: {:?}", trap_type);
            }
        }
        TrapType::SupervisorExternal => {
            get_int_device().try_handle_interrupt(u32::MAX);
        }
        _ => {
            // warn!("trap_type: {:?}  context: {:#x?}", trap_type, cx);
            // debug!("kernel_interrupt");
        }
    };
}
