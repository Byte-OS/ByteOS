use core::{future::Future, mem::size_of};

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use arch::{get_time, trap_pre_handle, user_restore, Context, ContextOps, VirtPage};
use executor::{
    current_task, current_user_task, thread, yield_now, Executor, KernelTask, MemType, UserTask,
};
use log::debug;
use signal::SignalFlags;

use crate::syscall::{c2rust_ref, consts::SignalUserContext, exec_with_process, syscall};

use self::initproc::initproc;

mod async_ops;
pub mod elf;
mod initproc;

pub use async_ops::{
    futex_requeue, futex_wake, NextTick, WaitFutex, WaitPid, WaitSignal, FUTEX_TABLE,
};

#[no_mangle]
// for avoiding the rust cycle check. user extern and nomangle
pub fn user_entry() -> Box<dyn Future<Output = ()> + Send + Sync> {
    Box::new(async { user_entry_inner().await })
}

enum UserTaskControlFlow {
    Continue,
    Break,
}

async fn handle_syscall(task: Arc<UserTask>, cx_ref: &mut Context) -> UserTaskControlFlow {
    let ustart = 0;
    unsafe {
        user_restore(cx_ref);
    }
    task.inner_map(|mut inner| inner.tms.utime += (get_time() - ustart) as u64);

    let sstart = 0;
    let trap_type = trap_pre_handle(cx_ref);
    match trap_type {
        arch::TrapType::Breakpoint => {}
        arch::TrapType::UserEnvCall => {
            debug!("user env call: {}", cx_ref.syscall_number());
            // if syscall ok
            let args = cx_ref.args();
            let args = [
                args[0], args[1], args[2], args[3], args[4], args[5], args[6],
            ];
            let call_number = cx_ref.syscall_number();
            cx_ref.syscall_ok();
            let result = syscall(call_number, args)
                .await
                .map_or_else(|e| -e.code(), |x| x as isize) as usize;
            debug!("syscall result: {:#X?}", result);
            cx_ref.set_ret(result);
            if result == (-500 as isize) as usize {
                return UserTaskControlFlow::Break;
            }
        }
        arch::TrapType::Time => {
            debug!("time interrupt from user");
        }
        arch::TrapType::Unknown => {
            debug!("unknown trap: {:#x?}", cx_ref);
            panic!("");
        }
        arch::TrapType::StorePageFault(addr) => {
            let vpn = VirtPage::from_addr(addr);
            debug!("store page fault @ {:#x}", addr);
            let mem_tracker = task
                .inner
                .lock()
                .memset
                .iter()
                .find(|x| {
                    x.vpn == vpn
                        && match x.mem_type {
                            MemType::Clone => true,
                            _ => false,
                        }
                })
                .map(|x| x.tracker.clone());

            match mem_tracker {
                Some(tracker) => {
                    let src_ppn = tracker.0;
                    let dst_ppn = task.frame_alloc(vpn, MemType::CodeSection);
                    dst_ppn.copy_value_from_another(src_ppn);
                }
                None => {
                    if (0x7fff0000..0x7ffff000).contains(&addr) {
                        task.frame_alloc(vpn, MemType::Stack);
                    } else {
                        debug!("context: {:#X?}", cx_ref);
                        return UserTaskControlFlow::Break;
                    }
                }
            }
        }
    }
    task.inner_map(|mut inner| inner.tms.stime += (get_time() - sstart) as u64);
    UserTaskControlFlow::Continue
}

pub async fn handle_signal(task: Arc<UserTask>, signal: SignalFlags) {
    let sigaction = task
        .inner_map(|inner| inner.sigaction.lock().get(signal.num()).unwrap().clone())
        .clone();

    if sigaction.handler == 0 {
        match signal {
            SignalFlags::SIGCANCEL => {
                current_user_task().exit_with_signal(signal.num());
            }
            _ => {}
        }
        return;
    }

    debug!("sigactions: {:#X?}", sigaction);

    let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };

    // let store_cx = cx_ref.clone();

    let mut sp = cx_ref.sp();

    sp -= 128;
    sp -= size_of::<SignalUserContext>();
    sp = sp / 16 * 16;

    let cx = c2rust_ref(sp as *mut SignalUserContext);
    let store_cx = cx_ref.clone();
    task.inner_map(|mut inner| {
        // cx.context.clone_from(&inner.cx);
        cx.pc = inner.cx.sepc();
        cx.sig_mask = sigaction.mask;
        debug!("pc: {:#X}, mask: {:#X?}", cx.pc, cx.sig_mask);
        inner.cx.set_sepc(sigaction.handler);
        inner.cx.set_ra(sigaction.restorer);
        inner.cx.set_arg0(signal.num());
        inner.cx.set_arg1(0);
        inner.cx.set_arg2(sp);
    });

    loop {
        if let UserTaskControlFlow::Break = handle_syscall(task.clone(), cx_ref).await {
            break;
        }
    }

    debug!("new pc: {:#X}", cx.pc);
    // store_cx.set_ret(cx_ref.args()[0]);
    cx_ref.clone_from(&store_cx);
    // copy pc from new_pc
    cx_ref.set_sepc(cx.pc);
}

pub async fn user_entry_inner() {
    let mut times = 0;
    loop {
        let task = current_user_task();
        debug!("user_entry, task: {}", task.task_id);
        loop {
            if let Some(signal) = task.inner_map(|mut x| x.signal.handle_signal()) {
                debug!("handle signal: {:?}  num: {}", signal, signal.num());
                handle_signal(task.clone(), signal.clone()).await;
            } else {
                break;
            }
        }
        let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };

        if let Some(exit_code) = task.exit_code() {
            debug!("program exit with code: {}", exit_code);
            break;
        }

        if let UserTaskControlFlow::Break = handle_syscall(task, cx_ref).await {
            break;
        }

        times += 1;

        if times >= 50 {
            times = 0;
            yield_now().await;
        }

        // yield_now().await;
    }
}

pub fn init() {
    let mut exec = Executor::new();
    exec.spawn(KernelTask::new(initproc()));
    // exec.spawn()
    exec.run();
}

pub async fn add_user_task(filename: &str, args: Vec<&str>, _envp: Vec<&str>) {
    let task = UserTask::new(user_entry(), Some(current_task()));
    exec_with_process(task.clone(), filename, args).expect("can't add task to excutor");
    thread::spawn(task.clone());
}
