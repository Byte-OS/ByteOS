use crate::syscall::types::signal::SignalUserContext;
use crate::tasks::current_user_task;
use core::mem::size_of;
use executor::AsyncTask;
use libc_types::internal::SigAction;
use libc_types::signal::SignalNum;
use log::debug;
use polyhal_trap::trapframe::TrapFrameArgs;

use super::UserTaskContainer;

impl UserTaskContainer {
    pub async fn handle_signal(&self, signal: SignalNum) {
        debug!(
            "handle signal: {:?} task_id: {}",
            signal,
            self.task.get_task_id()
        );

        // get the signal action for the signal.
        let sigaction = self.task.pcb.lock().sigaction[signal.num()].clone();

        if sigaction.handler == SigAction::SIG_IGN {
            // ignore signal if the handler of is SIG_IGN(1)
            return;
        } else if sigaction.handler == 0 || sigaction.handler == SigAction::SIG_DFL {
            // if there doesn't have signal handler.
            // Then use default handler. Exit or do nothing.
            if matches!(signal, SignalNum::CANCEL | SignalNum::SEGV | SignalNum::ILL) {
                current_user_task().exit_with_signal(signal.num());
            }
            return;
        }

        let cx_ref = self.task.force_cx_ref();
        // store task_mask and context.
        let task_mask = self.task.tcb.read().sigmask;
        self.task.tcb.write().sigmask = sigaction.mask;
        // alloc space for SignalUserContext at stack and align with 16 bytes.
        let sp = (cx_ref[TrapFrameArgs::SP] - size_of::<SignalUserContext>()) & !0xF;
        // let cx: &mut SignalUserContext = UserRef::<SignalUserContext>::from(sp).get_mut();
        let cx = unsafe { (sp as *mut SignalUserContext).as_mut().unwrap() };

        // change task context to do the signal.
        let mut tcb = self.task.tcb.write();
        cx.store_ctx(&cx_ref);
        cx.set_pc(tcb.cx[TrapFrameArgs::SEPC]);
        cx.set_sig_mask(sigaction.mask);
        tcb.cx[TrapFrameArgs::SP] = sp;
        tcb.cx[TrapFrameArgs::SEPC] = sigaction.handler;
        tcb.cx[TrapFrameArgs::RA] = sigaction.restorer;
        tcb.cx[TrapFrameArgs::ARG0] = signal.num();
        tcb.cx[TrapFrameArgs::ARG1] = 0;
        tcb.cx[TrapFrameArgs::ARG2] = cx as *mut SignalUserContext as usize;
        tcb.store_uctx.push_back((sp.into(), task_mask));
        drop(tcb);
    }
}
