use core::future::Future;

use alloc::boxed::Box;
use alloc::sync::Arc;
use arch::ContextOps;
use executor::signal::SignalList;
use executor::{current_task, current_user_task, yield_now, AsyncTask, UserTask};
use hal::TimeVal;
use log::debug;
use signal::{SigProcMask, SignalFlags};

use crate::tasks::user::{handle_user_interrupt, signal::handle_signal};
use crate::tasks::UserTaskControlFlow;

#[no_mangle]
// for avoiding the rust cycle check. use extern and nomangle
pub fn user_entry() -> Box<dyn Future<Output = ()> + Send + Sync> {
    Box::new(async { user_entry_inner().await })
}

pub fn check_timer(task: &Arc<UserTask>) {
    let mut pcb = task.pcb.lock();
    let timer = &mut pcb.timer[0];
    if timer.next > timer.last {
        let now = TimeVal::now();
        if now >= timer.next {
            task.tcb.write().signal.add_signal(SignalFlags::SIGALRM);
            timer.last = timer.next;
        }
    }
}

pub fn mask_signal_list(mask: SigProcMask, list: SignalList) -> SignalList {
    SignalList {
        signal: !mask.mask & list.signal,
    }
}

pub async fn user_entry_inner() {
    let mut times = 0;
    loop {
        let task = current_user_task();
        debug!("task: {}", task.get_task_id());

        // check timer
        check_timer(&task);

        loop {
            let sig_mask = task.tcb.read().sigmask;
            let signal =
                mask_signal_list(sig_mask, task.tcb.read().signal.clone()).try_get_signal();
            if let Some(signal) = signal {
                debug!("mask: {:?}", sig_mask);
                handle_signal(task.clone(), signal.clone()).await;
                task.tcb.write().signal.remove_signal(signal);
            } else {
                break;
            }
        }

        if let Some(exit_code) = task.exit_code() {
            debug!(
                "program exit with code: {}  task_id: {}  with  inner",
                exit_code,
                task.get_task_id()
            );
            break;
        }

        // let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };
        let cx_ref = task.force_cx_ref();
        debug!(
            "user_entry, task: {}, sepc: {:#X}",
            task.task_id,
            cx_ref.sepc()
        );

        if let UserTaskControlFlow::Break = handle_user_interrupt(task.clone(), cx_ref).await {
            break;
        }

        if let Some(exit_code) = task.exit_code() {
            debug!(
                "program exit with code: {}  task_id: {}  with  inner",
                exit_code,
                task.get_task_id()
            );
            break;
        }

        times += 1;

        if times >= 50 {
            times = 0;
            yield_now().await;
        }

        // yield_now().await;
    }
    debug!("exit_task: {}", current_task().get_task_id());
}
