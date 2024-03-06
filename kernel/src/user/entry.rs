use arch::{Context, ContextOps};
use executor::{current_user_task, yield_now, AsyncTask};
use futures_lite::future;
use hal::TimeVal;
use log::debug;
use signal::SignalFlags;

use crate::tasks::UserTaskControlFlow;

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
            let now = TimeVal::now();
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

    pub async fn entry_point(&mut self, cx_ref: &mut Context) {
        let mut times = 0;

        let check_signal = async || {
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
        };

        loop {
            self.check_timer();

            check_signal().await;

            // check for task exit status.
            if let Some(exit_code) = self.check_thread_exit() {
                debug!(
                    "program exit with code: {}  task_id: {}  with  inner",
                    exit_code,
                    self.task.get_task_id()
                );
                break;
            }

            debug!(
                "[task {}] user_entry sepc: {:#X}",
                self.task.task_id,
                cx_ref.sepc()
            );

            let res = future::or(self.handle_user_interrupt(cx_ref), async {
                loop {
                    check_signal().await;

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
    }
}

pub async fn user_entry() {
    let task = current_user_task();
    let cx_ref = task.force_cx_ref();
    let tid = task.get_task_id();
    UserTaskContainer {
        task,
        tid,
        store_frames: vec![],
    }
    .entry_point(cx_ref)
    .await;
}
