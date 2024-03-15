use core::pin::Pin;

use ::signal::SignalFlags;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use arch::{get_time, run_user_task, Context, ContextArgs, MappingFlags, VirtPage};
use executor::{AsyncTask, MapTrack, TaskId, UserTask};
use frame_allocator::frame_alloc;
use futures_lite::Future;
use log::{debug, warn};

use crate::{
    syscall::consts::SYS_SIGRETURN,
    tasks::{hexdump, UserTaskControlFlow},
};

pub mod entry;
pub mod signal;
pub mod socket_pair;

pub struct UserTaskContainer {
    pub task: Arc<UserTask>,
    pub tid: TaskId,
    pub store_frames: Vec<(Context, Pin<Box<dyn Future<Output = UserTaskControlFlow>>>)>,
}

/// Copy on write.
/// call this function when trigger store/instruction page fault.
/// copy page or remap page.
pub fn user_cow_int(task: Arc<UserTask>, _cx_ref: &mut Context, addr: usize) {
    let vpn = VirtPage::from_addr(addr);
    warn!(
        "store/instruction page fault @ {:#x} vaddr: {:#x} paddr: {:?} task_id: {}",
        addr,
        addr,
        task.page_table.virt_to_phys(addr.into()),
        task.get_task_id()
    );
    let mut pcb = task.pcb.lock();
    let area = pcb.memset.iter_mut().find(|x| x.contains(addr));
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
        task.map(ppn, vpn, MappingFlags::URWX);
    } else {
        task.tcb.write().signal.add_signal(SignalFlags::SIGSEGV);
    }
}

impl UserTaskContainer {
    /// Handle user interrupt.
    pub async fn handle_syscall(&self, cx_ref: &mut Context) -> UserTaskControlFlow {
        let ustart = get_time();
        if let Some(()) = run_user_task(cx_ref) {
            self.task
                .inner_map(|inner| inner.tms.utime += (get_time() - ustart) as u64);

            let sstart = get_time();
            if cx_ref[ContextArgs::SYSCALL] == SYS_SIGRETURN {
                return UserTaskControlFlow::Break;
            }

            debug!("syscall num: {}", cx_ref[ContextArgs::SYSCALL]);
            // sepc += 4, let it can go to next command.
            cx_ref.syscall_ok();
            let result = self
                .syscall(cx_ref[ContextArgs::SYSCALL], cx_ref.args())
                .await
                .map_or_else(|e| -e.code(), |x| x as isize) as usize;

            debug!(
                "[task {}] syscall result: {}",
                self.task.get_task_id(),
                result as isize
            );

            cx_ref[ContextArgs::RET] = result;
            self.task
                .inner_map(|inner| inner.tms.stime += (get_time() - sstart) as u64);
        }

        // let trap_type = trap_pre_handle(cx_ref);
        // match trap_type {
        //     arch::TrapType::Time => {
        //         // debug!("time interrupt from user");
        //     }
        //     arch::TrapType::Unknown => {
        //         debug!("unknown trap: {:#x?}", cx_ref);
        //         panic!("");
        //     }
        //     arch::TrapType::SupervisorExternal => {
        //         get_int_device().try_handle_interrupt(u32::MAX);
        //     }
        // }
        UserTaskControlFlow::Continue
    }
}

pub fn task_ilegal(task: &Arc<UserTask>, addr: usize, cx_ref: &mut Context) {
    let vpn = VirtPage::from_addr(addr);
    let mut pcb = task.pcb.lock();
    let area = pcb.memset.iter_mut().find(|x| x.contains(addr));
    if let Some(area) = area {
        let finded = area.mtrackers.iter_mut().find(|x| x.vpn == vpn);
        match finded {
            Some(_) => {
                cx_ref[ContextArgs::SEPC] += 2;
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
