use core::future::Future;

use alloc::boxed::Box;
use arch::ContextOps;
use executor::{current_task, current_user_task, yield_now, AsyncTask};
use log::debug;

use crate::tasks::user::{handle_user_interrupt, signal::handle_signal};
use crate::tasks::UserTaskControlFlow;

#[no_mangle]
// for avoiding the rust cycle check. use extern and nomangle
pub fn user_entry() -> Box<dyn Future<Output = ()> + Send + Sync> {
    Box::new(async { user_entry_inner().await })
}

pub async fn user_entry_inner() {
    let mut times = 0;
    loop {
        let task = current_user_task();
        debug!("task: {}", task.get_task_id());

        loop {
            let signal = task.tcb.read().signal.try_get_signal();
            if let Some(signal) = signal {
                handle_signal(task.clone(), signal.clone()).await;
                task.tcb.write().signal.remove_signal(signal);
            } else {
                break;
            }
        }

        let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };
        debug!(
            "user_entry, task: {}, sepc: {:#X}",
            task.task_id,
            cx_ref.sepc()
        );

        if let UserTaskControlFlow::Break = handle_user_interrupt(task.clone(), cx_ref).await {
            break;
        }

        if let Some(exit_code) = task.exit_code() {
            debug!("program exit with code: {}", exit_code);
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
