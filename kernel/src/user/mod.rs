use crate::tasks::UserTaskControlFlow;
use crate::tasks::{MapTrack, MemType, UserTask};
use crate::utils::hexdump;
use alloc::sync::Arc;
use devices::PAGE_SIZE;
use executor::{AsyncTask, TaskId};
use libc_types::signal::SignalNum;
use log::{debug, warn};
use polyhal::{MappingFlags, Time, VirtAddr};
use polyhal_trap::trap::{run_user_task, EscapeReason};
use polyhal_trap::trapframe::{TrapFrame, TrapFrameArgs};
use runtime::frame::frame_alloc;
use syscalls::Sysno;

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
pub fn user_cow_int(task: Arc<UserTask>, cx_ref: &mut TrapFrame, vaddr: VirtAddr) {
    warn!(
        "store/instruction page fault @ {:#x} vaddr: {} paddr: {:?} task_id: {}",
        cx_ref[TrapFrameArgs::SEPC],
        vaddr,
        task.page_table.translate(vaddr),
        task.get_task_id()
    );
    let mut pcb = task.pcb.lock();
    let area = pcb.memset.iter_mut().find(|x| x.contains(vaddr.raw()));
    if let Some(area) = area {
        let finded = area.mtrackers.iter_mut().find(|x| x.vaddr == vaddr.floor());
        let ppn = match finded {
            Some(map_track) => {
                if area.mtype == MemType::Shared {
                    task.tcb.write().signal.insert(SignalNum::SEGV);
                    return;
                }
                // tips: this finded will consume a strong count.
                debug!("strong count: {}", Arc::strong_count(&map_track.tracker));
                if Arc::strong_count(&map_track.tracker) > 1 {
                    let src = map_track.tracker.0;
                    let dst = frame_alloc().expect("can't alloc @ user page fault");
                    unsafe {
                        dst.0
                            .get_mut_ptr::<u8>()
                            .copy_from_nonoverlapping(src.get_ptr(), PAGE_SIZE);
                    }
                    map_track.tracker = Arc::new(dst);
                }
                map_track.tracker.0
            }
            None => {
                let tracker = Arc::new(frame_alloc().expect("can't alloc frame in cow_fork_int"));
                let mtracker = MapTrack {
                    vaddr: vaddr.floor(),
                    tracker,
                    rwx: 0b111,
                };
                let offset = vaddr.floor().raw() + area.offset - area.start;
                if let Some(file) = &area.file {
                    file.readat(offset, mtracker.tracker.0.slice_mut_with_len(PAGE_SIZE))
                        .expect("can't read file in cow_fork_int");
                }
                let ppn = mtracker.tracker.0;
                area.mtrackers.push(mtracker);
                ppn
            }
        };

        drop(pcb);
        task.map(ppn, vaddr.floor(), MappingFlags::URWX);
    } else {
        task.tcb.write().signal.insert(SignalNum::SEGV);
    }
}

impl UserTaskContainer {
    /// Handle user interrupt.
    pub async fn handle_syscall(&self, cx_ref: &mut TrapFrame) -> UserTaskControlFlow {
        let ustart = Time::now().raw();
        if matches!(run_user_task(cx_ref), EscapeReason::SysCall) {
            self.task
                .inner_map(|inner| inner.tms.utime += (Time::now().raw() - ustart) as u64);

            let sstart = Time::now().raw();
            if cx_ref[TrapFrameArgs::SYSCALL] == Sysno::rt_sigreturn.id() as _ {
                return UserTaskControlFlow::Break;
            }
            cx_ref.syscall_ok();
            let result = self
                .syscall(cx_ref[TrapFrameArgs::SYSCALL], cx_ref.args())
                .await
                .map_or_else(|e| -e.into_raw() as isize, |x| x as isize)
                as usize;

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

pub fn task_ilegal(task: &Arc<UserTask>, vaddr: VirtAddr, cx_ref: &mut TrapFrame) {
    let mut pcb = task.pcb.lock();
    let area = pcb.memset.iter_mut().find(|x| x.contains(vaddr.raw()));
    if let Some(area) = area {
        let finded = area.mtrackers.iter_mut().find(|x| x.vaddr == vaddr);
        match finded {
            Some(_) => {
                cx_ref[TrapFrameArgs::SEPC] += 2;
            }
            None => {
                task.tcb.write().signal.insert(SignalNum::ILL);
                unsafe {
                    hexdump(
                        core::slice::from_raw_parts_mut(vaddr.raw() as _, 0x1000),
                        vaddr.raw(),
                    );
                }
            }
        };
    } else {
        task.tcb.write().signal.insert(SignalNum::ILL);
        unsafe {
            hexdump(
                core::slice::from_raw_parts_mut(vaddr.raw() as _, 0x1000),
                vaddr.raw(),
            );
        }
    }
}
