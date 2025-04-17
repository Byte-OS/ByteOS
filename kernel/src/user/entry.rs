use alloc::boxed::Box;
use async_recursion::async_recursion;
use executor::{boot_page_table, yield_now, AsyncTask};
use futures_lite::future;
use log::debug;
use polyhal_trap::trapframe::TrapFrame;
use signal::SignalFlags;

use crate::{
    tasks::{current_user_task, UserTaskControlFlow},
    utils::time::current_timeval,
};

use super::UserTaskContainer;

impl UserTaskContainer {
    pub fn check_thread_exit(&self) -> Option<usize> {
        self.task
            .exit_code()
            .or(self.task.tcb.read().thread_exit_code.map(|x| x as usize))
    }

    pub fn check_timer(&self) {
        let mut pcb = self.task.pcb.lock();
        let timer = &mut pcb.timer[0];
        if timer.next > timer.last {
            log::error!("awake timer");
            let now = current_timeval();
            if now >= timer.next {
                self.task
                    .tcb
                    .write()
                    .signal
                    .add_signal(SignalFlags::SIGALRM);
                timer.last = timer.next;
            }
        }
    }

    pub async fn check_signal(&self) {
        loop {
            let sig_mask = self.task.tcb.read().sigmask;
            let signal = self
                .task
                .tcb
                .read()
                .signal
                .clone()
                .mask(sig_mask)
                .try_get_signal();
            if let Some(signal) = signal {
                debug!("mask: {:?}", sig_mask);
                self.handle_signal(signal.clone()).await;
                let mut tcb = self.task.tcb.write();
                tcb.signal.remove_signal(signal.clone());
                // check if it is a real time signal
                if let Some(index) = signal.real_time_index()
                    && tcb.signal_queue[index] > 0
                {
                    tcb.signal.add_signal(signal.clone());
                    tcb.signal_queue[index] -= 1;
                }
            } else {
                break;
            }
        }
    }

    pub async fn entry_point(&mut self, cx_ref: &mut TrapFrame) {
        let mut times: i32 = 0;

        loop {
            self.check_timer();
            self.check_signal().await;

            // check for task exit status.
            if let Some(exit_code) = self.check_thread_exit() {
                debug!(
                    "program exit with code: {}  task_id: {}  with  inner",
                    exit_code,
                    self.task.get_task_id()
                );
                break;
            }

            let res = future::or(self.handle_syscall(cx_ref), async {
                loop {
                    self.check_signal().await;

                    if let Some(_exit_code) = self.check_thread_exit() {
                        return UserTaskControlFlow::Break;
                    }
                    self.check_timer();
                    yield_now().await;
                }
            });

            if let UserTaskControlFlow::Break = res.await {
                break;
            }

            if let Some(exit_code) = self.check_thread_exit() {
                debug!(
                    "program exit with code: {}  task_id: {}  with  inner",
                    exit_code,
                    self.task.get_task_id()
                );
                break;
            }

            times += 1;
            if times >= 50 {
                times = 0;
                yield_now().await;
            }
        }

        debug!("exit_task: {}", self.task.get_task_id());
        boot_page_table().change();
    }
}

#[async_recursion(Sync)]
pub async fn user_entry() {
    let task = current_user_task();
    let cx_ref = task.force_cx_ref();
    let tid = task.get_task_id();
    UserTaskContainer { task, tid }.entry_point(cx_ref).await;
}
