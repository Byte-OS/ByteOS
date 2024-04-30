use ::signal::SignalFlags;
use alloc::sync::Arc;
use polyhal::addr::VirtPage;
use polyhal::{pagetable::MappingFlags, run_user_task, time::Time, TrapFrame, TrapFrameArgs};
use executor::{AsyncTask, TaskId};
use frame_allocator::frame_alloc;
use log::{debug, warn};

use crate::tasks::{MapTrack, MemType, UserTask};
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
}

/// Copy on write.
/// call this function when trigger store/instruction page fault.
/// copy page or remap page.
pub fn user_cow_int(task: Arc<UserTask>, _cx_ref: &mut TrapFrame, addr: usize) {
    let vpn = VirtPage::from_addr(addr);
    warn!(
        "store/instruction page fault @ {:#x} vaddr: {:#x} paddr: {:?} task_id: {}",
        addr,
        addr,
        task.page_table.translate(addr.into()),
        task.get_task_id()
    );
    let mut pcb = task.pcb.lock();
    let area = pcb.memset.iter_mut().find(|x| x.contains(addr));
    if let Some(area) = area {
        let finded = area.mtrackers.iter_mut().find(|x| x.vpn == vpn);
        let ppn = match finded {
            Some(map_track) => {
                if area.mtype == MemType::Shared {
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
    pub async fn handle_syscall(&self, cx_ref: &mut TrapFrame) -> UserTaskControlFlow {
        let ustart = Time::now().raw();
        if let Some(()) = run_user_task(cx_ref) {
            self.task
                .inner_map(|inner| inner.tms.utime += (Time::now().raw() - ustart) as u64);

            let sstart = Time::now().raw();
            if cx_ref[TrapFrameArgs::SYSCALL] == SYS_SIGRETURN {
                return UserTaskControlFlow::Break;
            }

            debug!("syscall num: {}", cx_ref[TrapFrameArgs::SYSCALL]);
            // sepc += 4, let it can go to next command.
            cx_ref.syscall_ok();
            let result = self
                .syscall(cx_ref[TrapFrameArgs::SYSCALL], cx_ref.args())
                .await
                .map_or_else(|e| -e.code(), |x| x as isize) as usize;

            debug!(
                "[task {}] syscall result: {}",
                self.task.get_task_id(),
                result as isize
            );

            cx_ref[TrapFrameArgs::RET] = result;
            self.task
                .inner_map(|inner| inner.tms.stime += (Time::now().raw() - sstart) as u64);
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

pub fn task_ilegal(task: &Arc<UserTask>, addr: usize, cx_ref: &mut TrapFrame) {
    let vpn = VirtPage::from_addr(addr);
    let mut pcb = task.pcb.lock();
    let area = pcb.memset.iter_mut().find(|x| x.contains(addr));
    if let Some(area) = area {
        let finded = area.mtrackers.iter_mut().find(|x| x.vpn == vpn);
        match finded {
            Some(_) => {
                cx_ref[TrapFrameArgs::SEPC] += 2;
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
