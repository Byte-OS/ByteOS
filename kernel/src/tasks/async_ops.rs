use core::{cmp, future::Future, pin::Pin, task::Poll};

use alloc::{sync::Arc, vec::Vec};
use arch::get_time_ms;
use executor::{current_user_task, FutexTable, UserTask};
use sync::Mutex;

use crate::syscall::consts::LinuxError;

pub struct NextTick(usize);

impl Future for NextTick {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let curr = get_time_ms();
        if curr < self.0 {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub struct WaitPid(pub Arc<UserTask>, pub isize);

impl Future for WaitPid {
    type Output = Arc<UserTask>;

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let inner = self.0.inner.lock();
        let res = inner.children.iter().find(|x| {
            let inner = x.inner.lock();
            (self.1 == -1 || x.task_id == self.1 as usize) && inner.exit_code.is_some()
        });
        match res {
            Some(task) => Poll::Ready(task.clone()),
            None => Poll::Pending,
        }
    }
}

pub struct WaitSignal(pub Arc<UserTask>);

impl Future for WaitSignal {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        match self.0.inner_map(|x| x.signal.has_signal()) {
            true => Poll::Ready(()),
            false => Poll::Pending,
        }
    }
}

pub fn in_futex(futex_table: Arc<Mutex<FutexTable>>, task_id: usize) -> bool {
    let futex_table = futex_table.lock();
    futex_table
        .values()
        .find(|x| x.contains(&task_id))
        .is_some()
}

pub struct WaitFutex(pub Arc<Mutex<FutexTable>>, pub usize);

impl Future for WaitFutex {
    type Output = Result<usize, LinuxError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let signal = current_user_task().inner_map(|inner| inner.signal.clone());
        match in_futex(self.0.clone(), self.1) {
            true => {
                if signal.has_signal() {
                    self.0
                        .lock()
                        .values_mut()
                        .find(|x| x.contains(&self.1))
                        .map(|x| x.retain(|x| *x != self.1));
                    Poll::Ready(Err(LinuxError::EINTR))
                } else {
                    Poll::Pending
                }
            }
            false => Poll::Ready(Ok(0)),
        }
    }
}

#[no_mangle]
pub fn futex_wake(futex_table: Arc<Mutex<FutexTable>>, uaddr: usize, wake_count: usize) -> usize {
    let mut futex_table = futex_table.lock();
    let que_size = futex_table.get_mut(&uaddr).map(|x| x.len()).unwrap_or(0);
    if que_size == 0 {
        0
    } else {
        let que = futex_table
            .get_mut(&uaddr)
            .map(|x| x.drain(..cmp::min(wake_count as usize, que_size)));

        que.map(|x| x.count()).unwrap_or(0)
    }
}

pub fn futex_requeue(
    futex_table: Arc<Mutex<FutexTable>>,
    uaddr: usize,
    wake_count: usize,
    uaddr2: usize,
    reque_count: usize,
) -> usize {
    let mut futex_table = futex_table.lock();

    let waked_size = futex_table
        .get_mut(&uaddr)
        .map(|x| x.drain(..wake_count as usize).count())
        .unwrap_or(0);

    let reque: Option<Vec<_>> = futex_table
        .get_mut(&uaddr)
        .map(|x| x.drain(..reque_count as usize).collect());

    if let Some(reque) = reque {
        if !futex_table.contains_key(&uaddr2) {
            futex_table.insert(uaddr2, vec![]);
        }
        futex_table.get_mut(&uaddr2).unwrap().extend(reque);
    }

    waked_size
}
