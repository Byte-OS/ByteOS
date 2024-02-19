use core::mem::size_of;

use alloc::sync::Arc;
use arch::{ContextOps, SIG_RETURN_ADDR};
use executor::{current_user_task, AsyncTask, UserTask};
use log::debug;
use signal::SignalFlags;

use crate::syscall::consts::{SignalUserContext, UserRef};
use crate::user::handle_user_interrupt;
use crate::tasks::UserTaskControlFlow;

pub async fn handle_signal(task: Arc<UserTask>, signal: SignalFlags) {
    debug!(
        "handle signal: {:?} task_id: {}",
        signal,
        task.get_task_id()
    );

    // if the signal is SIGKILL, then exit the task immediately.
    // the SIGKILL can't be catched and be ignored.
    if signal == SignalFlags::SIGKILL {
        task.exit_with_signal(signal.num());
    }

    // get the signal action for the signal.
    let sigaction = task.pcb.lock().sigaction[signal.num()].clone();

    // if there doesn't have signal handler.
    // Then use default handler. Exit or do nothing.
    // SIG_ERR = -1, SIG_DEF(default) = 0, SIG_IGN = 1(ignore)
    if sigaction.handler == 0 {
        match signal {
            SignalFlags::SIGCANCEL | SignalFlags::SIGSEGV | SignalFlags::SIGILL => {
                current_user_task().exit_with_signal(signal.num());
            }
            _ => {}
        }
        return;
    }
    // ignore signal if the handler of is SIG_IGN(1)
    if sigaction.handler == 1 {
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
    if sigaction.restorer == 0 {
        tcb.cx.set_ra(SIG_RETURN_ADDR);
    }
    tcb.cx.set_arg0(signal.num());
    tcb.cx.set_arg1(0);
    tcb.cx.set_arg2(sp);
    // info!("context: {:#X?}", tcb.cx);
    drop(tcb);

    loop {
        if let Some(exit_code) = task.exit_code() {
            debug!(
                "program exit with code: {}  task_id: {}",
                exit_code,
                task.get_task_id()
            );
            break;
        }

        // let cx_ref = unsafe {
        //     task.get_cx_ptr().as_mut().unwrap()
        // };
        let cx_ref = task.force_cx_ref();

        debug!("[task {}]task sepc: {:#x}", task.get_task_id(), cx_ref.sepc());

        if let UserTaskControlFlow::Break = handle_user_interrupt(task.clone(), cx_ref).await {
            break;
        }
    }
    info!(
        "handle signal: {:?} task: {} ended",
        signal,
        task.get_task_id()
    );
    // restore sigmask to the mask before doing the signal.
    task.tcb.write().sigmask = task_mask;
    // store_cx.set_ret(cx_ref.args()[0]);
    cx_ref.clone_from(&store_cx);
    // copy pc from new_pc
    cx_ref.set_sepc(cx.pc);
}
