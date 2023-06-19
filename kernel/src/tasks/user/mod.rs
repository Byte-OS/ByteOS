use frame_allocator::frame_alloc;
use ::signal::SignalFlags;
use alloc::sync::Arc;
use arch::{get_time, trap_pre_handle, user_restore, Context, ContextOps, VirtPage, PTEFlags};
use executor::{MemType, UserTask};
use log::{debug, warn};

use crate::syscall::{consts::SYS_SIGRETURN, syscall};

use super::UserTaskControlFlow;

pub mod entry;
pub mod signal;

/// Copy on write.
/// call this function when trigger store/instruction page fault.
/// copy page or remap page.
pub fn user_cow_int(task: Arc<UserTask>, cx_ref: &mut Context, addr: usize) {
    let vpn = VirtPage::from_addr(addr);
    warn!("store/intruction page fault @ {:#x} vpn: {}", addr, vpn);
    // warn!("user_task map: {:#x?}", task.pcb.lock().memset);
    let mut pcb = task.pcb.lock();
    let finded = pcb
        .memset
        .iter_mut()
        .find_map(|mem_area| {
            mem_area
                .mtrackers
                .iter_mut()
                // .find(|x| x.vpn == vpn && mem_area.mtype == MemType::Clone)
                .find(|x| x.vpn == vpn)
        });

    match finded {
        Some(map_track) => {
            // tips: this finded will consume a strong count.
            debug!("strong count: {}", Arc::strong_count(&map_track.tracker));
            if Arc::strong_count(&map_track.tracker) > 1  {
                let src_ppn = map_track.tracker.0;
                let dst_ppn = frame_alloc().expect("can't alloc @ user page fault");
                dst_ppn.0.copy_value_from_another(src_ppn);
                map_track.tracker = Arc::new(dst_ppn);
                task.map(map_track.tracker.0, map_track.vpn, PTEFlags::UVRWX);
            } else {
                task.map(map_track.tracker.0, map_track.vpn, PTEFlags::UVRWX);
            }
        }
        None => {
            if (0x7ff00000..0x7ffff000).contains(&addr) {
                task.frame_alloc(vpn, MemType::Stack, 1);
            } else {
                warn!("task exit with page fault, its context: {:#X?}", cx_ref);
                drop(pcb);
                task.exit_with_signal(SignalFlags::SIGABRT.num());
                debug!("exit");
            }
        }
    }
}

/// Handle user interrupt.
pub async fn handle_user_interrupt(
    task: Arc<UserTask>,
    cx_ref: &mut Context,
) -> UserTaskControlFlow {
    let ustart = 0;
    unsafe {
        user_restore(cx_ref);
    }
    task.inner_map(|inner| inner.tms.utime += (get_time() - ustart) as u64);

    let sstart = 0;
    let trap_type = trap_pre_handle(cx_ref);
    match trap_type {
        arch::TrapType::Breakpoint => {}
        arch::TrapType::UserEnvCall => {
            // if it is sigreturn then break the control flow.
            if cx_ref.syscall_number() == SYS_SIGRETURN {
                return UserTaskControlFlow::Break;
            }
            // sepc += 4, let it can go to next command.
            cx_ref.syscall_ok();
            let result = syscall(cx_ref.syscall_number(), cx_ref.args())
                .await
                .map_or_else(|e| -e.code(), |x| x as isize) as usize;

            debug!("syscall result: {:#X?}", result);

            cx_ref.set_ret(result);
        }
        arch::TrapType::Time => {
            debug!("time interrupt from user");
        }
        arch::TrapType::Unknown => {
            debug!("unknown trap: {:#x?}", cx_ref);
            panic!("");
        }
        arch::TrapType::IllegalInstruction(addr) => {
            let vpn = VirtPage::from_addr(addr);
            warn!("store/intruction page fault @ {:#x} vpn: {}", addr, vpn);
            warn!("the fault occurs @ {:#x}", cx_ref.sepc());
            warn!("user_task map: {:#x?}", task.pcb.lock().memset);
            warn!("mapped ppn addr: {:#x} @ {}", cx_ref.sepc(), task.page_table.virt_to_phys(cx_ref.sepc().into()));
            panic!("illegal Instruction")
        }
        arch::TrapType::StorePageFault(addr) | arch::TrapType::InstructionPageFault(addr) => user_cow_int(task.clone(), cx_ref, addr),
    }
    task.inner_map(|inner| inner.tms.stime += (get_time() - sstart) as u64);
    UserTaskControlFlow::Continue
}
