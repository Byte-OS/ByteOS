use alloc::sync::Arc;
use arch::ContextOps;
use executor::signal::SignalList;
use executor::{current_task, current_user_task, yield_now, AsyncTask, UserTask};
use futures_lite::future;
use hal::TimeVal;
use log::debug;
use signal::{SigProcMask, SignalFlags};

use crate::tasks::user::{handle_user_interrupt, signal::handle_signal};
use crate::tasks::UserTaskControlFlow;

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

#[inline]
pub fn check_thread_exit(task: &Arc<UserTask>) -> Option<usize> {
    task.exit_code()
        .or(task.tcb.read().thread_exit_code.map(|x| x as usize))
    // task.exit_code().is_some() || task.tcb.read().thread_exit_code.is_some()
}

pub async fn user_entry() {
    let task = current_user_task();
    let cx_ref = task.force_cx_ref();
    let mut times = 0;

    let check_signal = async || {
        loop {
            let sig_mask = task.tcb.read().sigmask;
            let signal =
                mask_signal_list(sig_mask, task.tcb.read().signal.clone()).try_get_signal();
            if let Some(signal) = signal {
                debug!("mask: {:?}", sig_mask);
                handle_signal(task.clone(), signal.clone()).await;
                let mut tcb = task.tcb.write();
                tcb.signal.remove_signal(signal.clone());
                // check if it is a real time signal
                if let Some(index) = signal.real_time_index() && tcb.signal_queue[index] > 0 {
                    tcb.signal.add_signal(signal.clone());
                    tcb.signal_queue[index] -= 1;
                }
            } else {
                break;
            }
        }
    };

    loop {
        check_timer(&task);

        check_signal().await;

        // check for task exit status.
        if let Some(exit_code) = check_thread_exit(&task) {
            debug!(
                "program exit with code: {}  task_id: {}  with  inner",
                exit_code,
                task.get_task_id()
            );
            break;
        }

        debug!(
            "[task {}] user_entry sepc: {:#X}",
            task.task_id,
            cx_ref.sepc()
        );

        let res = future::or(handle_user_interrupt(task.clone(), cx_ref), async {
            loop {
                check_signal().await;

                if let Some(_exit_code) = check_thread_exit(&task) {
                    return UserTaskControlFlow::Break;
                }
                check_timer(&task);
                yield_now().await;
            }
        });

        // let res = loop {
        //     match futures_lite::future::poll_once(handle_user_interrupt(task.clone(), cx_ref)).await {
        //         Some(result) => break result,
        //         None => {
        //             check_timer(&task);
        //             check_signal().await;

        //             if let Some(_exit_code) = task.exit_code() {
        //                 return;
        //             }
        //         },
        //     }
        // };

        if let UserTaskControlFlow::Break = res.await {
            break;
        }

        // if let UserTaskControlFlow::Break = handle_user_interrupt(task.clone(), cx_ref).await {
        //     break;
        // }

        if let Some(exit_code) = check_thread_exit(&task) {
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
    }

    // loop {
    //     // check timer
    //     check_timer(&task);

    //     loop {
    //         let sig_mask = task.tcb.read().sigmask;
    //         let signal =
    //             mask_signal_list(sig_mask, task.tcb.read().signal.clone()).try_get_signal();
    //         if let Some(signal) = signal {
    //             debug!("mask: {:?}", sig_mask);
    //             handle_signal(task.clone(), signal.clone()).await;
    //             task.tcb.write().signal.remove_signal(signal);
    //         } else {
    //             break;
    //         }
    //     }

    //     if let Some(exit_code) = task.exit_code() {
    //         debug!(
    //             "program exit with code: {}  task_id: {}  with  inner",
    //             exit_code,
    //             task.get_task_id()
    //         );
    //         break;
    //     }

    //     // let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };
    //     let cx_ref = task.force_cx_ref();
    //     debug!(
    //         "[task {}] user_entry sepc: {:#X}",
    //         task.task_id,
    //         cx_ref.sepc()
    //     );

    //     if let UserTaskControlFlow::Break = handle_user_interrupt(task.clone(), cx_ref).await {
    //         break;
    //     }

    //     if let Some(exit_code) = task.exit_code() {
    //         debug!(
    //             "program exit with code: {}  task_id: {}  with  inner",
    //             exit_code,
    //             task.get_task_id()
    //         );
    //         break;
    //     }

    //     times += 1;

    //     if times >= 50 {
    //         times = 0;
    //         yield_now().await;
    //     }

    //     // yield_now().await;
    // }
    debug!("exit_task: {}", current_task().get_task_id());
}
