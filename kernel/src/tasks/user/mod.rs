use ::signal::SignalFlags;
use alloc::sync::Arc;
use arch::{get_time, trap_pre_handle, user_restore, Context, ContextOps, PTEFlags, VirtPage};
use devices::get_int_device;
use executor::{AsyncTask, MapTrack, MemType, UserTask};
use frame_allocator::frame_alloc;
use log::{debug, warn};

use crate::{
    syscall::{consts::SYS_SIGRETURN, syscall},
    tasks::hexdump,
};

use super::UserTaskControlFlow;

pub mod entry;
pub mod signal;

/// Copy on write.
/// call this function when trigger store/instruction page fault.
/// copy page or remap page.
pub fn user_cow_int(task: Arc<UserTask>, _cx_ref: &mut Context, addr: usize) {
    let vpn = VirtPage::from_addr(addr);
    warn!(
        "store/instruction page fault @ {:#x} vpn: {} ppn: {} task_id: {}",
        addr,
        vpn,
        task.page_table.virt_to_phys(addr.into()),
        task.get_task_id()
    );
    let mut pcb = task.pcb.lock();
    let area = pcb
        .memset
        .iter_mut()
        .filter(|x| x.mtype != MemType::PTE)
        .find(|x| x.contains(addr));
    if let Some(area) = area {
        let finded = area.mtrackers.iter_mut().find(|x| x.vpn == vpn);
        let ppn = match finded {
            Some(map_track) => {
                if area.mtype == executor::MemType::Shared {
                    task.tcb.write().signal.add_signal(SignalFlags::SIGSEGV);
                    return;
                }
                // tips: this finded will consume a strong count.
                debug!("strong count: {}", Arc::strong_count(&map_track.tracker));
                if Arc::strong_count(&map_track.tracker) > 1 {
                    let src_ppn = map_track.tracker.0;
                    let dst_ppn = frame_alloc().expect("can't alloc @ user page fault");
                    dst_ppn.0.copy_value_from_another(src_ppn);
                    map_track.tracker = Arc::new(dst_ppn);
                }
                map_track.tracker.0
            }
            None => {
                let tracker = Arc::new(frame_alloc().expect("can't alloc frame in cow_fork_int"));
                let mtracker = MapTrack {
                    vpn,
                    tracker,
                    rwx: 0b111,
                };
                // mtracker.tracker.0.get_buffer().fill(0);
                let offset = vpn.to_addr() + area.offset - area.start;
                if let Some(file) = &area.file {
                    file.readat(offset, mtracker.tracker.0.get_buffer())
                        .expect("can't read file in cow_fork_int");
                }
                let ppn = mtracker.tracker.0;
                area.mtrackers.push(mtracker);
                ppn
            }
        };
        drop(pcb);
        task.map(ppn, vpn, PTEFlags::UVRWX);
    } else {
        task.tcb.write().signal.add_signal(SignalFlags::SIGSEGV);
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

            debug!("syscall num: {}", cx_ref.syscall_number());
            // sepc += 4, let it can go to next command.
            cx_ref.syscall_ok();
            let result = syscall(cx_ref.syscall_number(), cx_ref.args())
                .await
                .map_or_else(|e| -e.code(), |x| x as isize) as usize;

            debug!(
                "[task {}] syscall result: {}",
                task.get_task_id(),
                result as isize
            );

            cx_ref.set_ret(result);
        }
        arch::TrapType::Time => {
            // debug!("time interrupt from user");
        }
        arch::TrapType::Unknown => {
            debug!("unknown trap: {:#x?}", cx_ref);
            panic!("");
        }
        arch::TrapType::SupervisorExternal => {
            get_int_device().try_handle_interrupt(u32::MAX);
        }
        arch::TrapType::IllegalInstruction(addr) => {
            let vpn = VirtPage::from_addr(addr);
            warn!(
                "store/instruction page fault @ {:#x} vpn: {} ppn: {}",
                addr,
                vpn,
                task.page_table.virt_to_phys(addr.into()),
            );
            warn!("the fault occurs @ {:#x}", cx_ref.sepc());
            // warn!("user_task map: {:#x?}", task.pcb.lock().memset);
            warn!(
                "mapped ppn addr: {:#x} @ {}",
                cx_ref.sepc(),
                task.page_table.virt_to_phys(cx_ref.sepc().into())
            );

            task_ilegal(&task, cx_ref.sepc(), cx_ref);
            // panic!("illegal Instruction")
            // let signal = task.tcb.read().signal.clone();
            // if signal.has_sig(SignalFlags::SIGSEGV) {
            //     task.exit_with_signal(SignalFlags::SIGSEGV.num());
            // } else {
            //     return UserTaskControlFlow::Break
            // }
            // current_user_task()
            //     .tcb
            //     .write()
            //     .signal
            //     .add_signal(SignalFlags::SIGSEGV);
            // return UserTaskControlFlow::Break;
        }
        arch::TrapType::StorePageFault(addr)
        | arch::TrapType::InstructionPageFault(addr)
        | arch::TrapType::LoadPageFault(addr) => {
            debug!("store page fault");
            user_cow_int(task.clone(), cx_ref, addr)
        }
    }
    task.inner_map(|inner| inner.tms.stime += (get_time() - sstart) as u64);
    UserTaskControlFlow::Continue
}

pub fn task_ilegal(task: &Arc<UserTask>, addr: usize, cx_ref: &mut Context) {
    let vpn = VirtPage::from_addr(addr);
    let mut pcb = task.pcb.lock();
    let area = pcb
        .memset
        .iter_mut()
        .filter(|x| x.mtype != MemType::PTE)
        .find(|x| x.contains(addr));
    if let Some(area) = area {
        let finded = area.mtrackers.iter_mut().find(|x| x.vpn == vpn);
        match finded {
            Some(_) => {
                cx_ref.set_sepc(cx_ref.sepc() + 2);
            }
            None => {
                task.tcb.write().signal.add_signal(SignalFlags::SIGILL);
                unsafe {
                    hexdump(
                        core::slice::from_raw_parts_mut(vpn.to_addr() as _, 0x1000),
                        vpn.to_addr(),
                    );
                }
            }
        };
    } else {
        task.tcb.write().signal.add_signal(SignalFlags::SIGILL);
        unsafe {
            hexdump(
                core::slice::from_raw_parts_mut(vpn.to_addr() as _, 0x1000),
                vpn.to_addr(),
            );
        }
    }
}
