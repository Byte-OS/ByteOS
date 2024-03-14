use core::mem::size_of;

use arch::{ContextArgs, SIG_RETURN_ADDR};
use executor::{current_user_task, AsyncTask};
use log::debug;
use signal::SignalFlags;

use crate::syscall::consts::{SignalUserContext, UserRef};
use crate::tasks::UserTaskControlFlow;

use super::UserTaskContainer;

impl UserTaskContainer {
    pub async fn handle_signal(&self, signal: SignalFlags) {
        debug!(
            "handle signal: {:?} task_id: {}",
            signal,
            self.task.get_task_id()
        );

        // if the signal is SIGKILL, then exit the task immediately.
        // the SIGKILL can't be catched and be ignored.
        if signal == SignalFlags::SIGKILL {
            self.task.exit_with_signal(signal.num());
        }

        // get the signal action for the signal.
        let sigaction = self.task.pcb.lock().sigaction[signal.num()].clone();

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

        info!(
            "handle signal: {:?} task: {}",
            signal,
            self.task.get_task_id()
        );

        // let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };
        let cx_ref = self.task.force_cx_ref();
        // store task_mask and context.
        let task_mask = self.task.tcb.read().sigmask;
        let store_cx = cx_ref.clone();
        self.task.tcb.write().sigmask = sigaction.mask;

        // alloc space for SignalUserContext at stack and align with 16 bytes.
        let sp = (cx_ref[ContextArgs::SP] - 128 - size_of::<SignalUserContext>()) / 16 * 16;
        let cx: &mut SignalUserContext = UserRef::<SignalUserContext>::from(sp).get_mut();
        // change task context to do the signal.
        let mut tcb = self.task.tcb.write();
        cx.store_ctx(&cx_ref);
        cx.set_pc(tcb.cx[ContextArgs::SEPC]);
        cx.sig_mask = sigaction.mask;
        tcb.cx[ContextArgs::SP] = sp;
        tcb.cx[ContextArgs::SEPC] = sigaction.handler;
        tcb.cx[ContextArgs::RA] = if sigaction.restorer == 0 {
             SIG_RETURN_ADDR
        } else {
            sigaction.restorer
        };
        tcb.cx[ContextArgs::ARG0] = signal.num();
        tcb.cx[ContextArgs::ARG1] = 0;
        tcb.cx[ContextArgs::ARG2] = cx as *mut SignalUserContext as usize;
        // info!("context: {:#X?}", tcb.cx);
        drop(tcb);

        loop {
            if let Some(exit_code) = self.task.exit_code() {
                debug!(
                    "program exit with code: {}  task_id: {}",
                    exit_code,
                    self.task.get_task_id()
                );
                break;
            }

            // let cx_ref = unsafe {
            //     task.get_cx_ptr().as_mut().unwrap()
            // };
            let cx_ref = self.task.force_cx_ref();

            debug!(
                "[task {}]task sepc: {:#x}",
                self.task.get_task_id(),
                cx_ref[ContextArgs::SEPC]
            );

            if let UserTaskControlFlow::Break = self.handle_syscall(cx_ref).await {
                break;
            }
        }
        info!(
            "handle signal: {:?} task: {} ended",
            signal,
            self.task.get_task_id()
        );
        // restore sigmask to the mask before doing the signal.
        self.task.tcb.write().sigmask = task_mask;
        // store_cx.set_ret(cx_ref.args()[0]);
        *cx_ref = store_cx;
        // copy pc from new_pc
        cx_ref[ContextArgs::SEPC] = cx.pc();
        cx.restore_ctx(cx_ref);
    }
}
