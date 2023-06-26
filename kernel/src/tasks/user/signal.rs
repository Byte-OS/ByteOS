use core::mem::size_of;

use alloc::sync::Arc;
use arch::ContextOps;
use executor::{current_user_task, AsyncTask, UserTask};
use log::debug;
use signal::SignalFlags;

use crate::syscall::consts::{SignalUserContext, UserRef};
use crate::tasks::user::handle_user_interrupt;
use crate::tasks::UserTaskControlFlow;

pub async fn handle_signal(task: Arc<UserTask>, signal: SignalFlags) {
    debug!(
        "handle signal: {:?} task_id: {}",
        signal,
        task.get_task_id()
    );

    // get the signal action for the signal.
    let sigaction = task.pcb.lock().sigaction[signal.num()].clone();

    // if there doesn't have signal handler.
    // Then use default handler. Exit or do nothing.
    if sigaction.handler == 0 {
        match signal {
            SignalFlags::SIGCANCEL | SignalFlags::SIGSEGV => {
                current_user_task().exit_with_signal(signal.num());
            }
            _ => {}
        }
        return;
    }

    info!("handle signal: {:?} task: {}", signal, task.get_task_id());

    // let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };
    let cx_ref = task.force_cx_ref();
    // store task_mask and context.
    let task_mask = task.tcb.read().sigmask;
    let store_cx = cx_ref.clone();
    task.tcb.write().sigmask = sigaction.mask;

    // alloc space for SignalUserContext at stack and align with 16 bytes.
    let sp = (cx_ref.sp() - 128 - size_of::<SignalUserContext>()) / 16 * 16;
    let cx = UserRef::<SignalUserContext>::from(sp).get_mut();
    // change task context to do the signal.
    let mut tcb = task.tcb.write();
    cx.pc = tcb.cx.sepc();
    cx.sig_mask = sigaction.mask;
    tcb.cx.set_sp(sp);
    tcb.cx.set_sepc(sigaction.handler);
    tcb.cx.set_ra(sigaction.restorer);
    tcb.cx.set_arg0(signal.num());
    tcb.cx.set_arg1(0);
    tcb.cx.set_arg2(sp);
    info!("context: {:#X?}", tcb.cx);
    drop(tcb);

    loop {
        if let Some(exit_code) = task.exit_code() {
            debug!("program exit with code: {}", exit_code);
            break;
        }

        // let cx_ref = unsafe {
        //     task.get_cx_ptr().as_mut().unwrap()
        // };
        let cx_ref = task.force_cx_ref();

        debug!("task sepc: {:#x}", cx_ref.sepc);

        if let UserTaskControlFlow::Break = handle_user_interrupt(task.clone(), cx_ref).await {
            break;
        }
    }
    // restore sigmask to the mask before doing the signal.
    task.tcb.write().sigmask = task_mask;
    // store_cx.set_ret(cx_ref.args()[0]);
    cx_ref.clone_from(&store_cx);
    // copy pc from new_pc
    cx_ref.set_sepc(cx.pc);
}
