use core::{future::Future, pin::Pin, task::Poll};

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use arch::get_time_ms;
use executor::UserTask;
use sync::Mutex;

pub static FUTEX_TABLE: Mutex<BTreeMap<usize, Vec<usize>>> = Mutex::new(BTreeMap::new());

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

pub fn in_futex(task_id: usize) -> bool {
    let futex_table = FUTEX_TABLE.lock();
    futex_table
        .values()
        .find(|x| x.contains(&task_id))
        .is_some()
}

pub struct WaitFutex(pub usize);

impl Future for WaitFutex {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        match in_futex(self.0) {
            true => Poll::Pending,
            false => Poll::Ready(()),
        }
    }
}

#[no_mangle]
pub fn futex_wake(uaddr: usize, wake_count: usize) -> usize {
    let mut futex_table = FUTEX_TABLE.lock();
    let que_size = futex_table.get_mut(&uaddr).map(|x| x.len()).unwrap_or(0);
    if que_size == 0 {
        0
    } else {
        let que = futex_table
            .get_mut(&uaddr)
            .map(|x| x.drain(..wake_count as usize));

        que.map(|x| x.count()).unwrap_or(0)
    }
}

pub fn futex_requeue(uaddr: usize, wake_count: usize, uaddr2: usize, reque_count: usize) -> usize {
    let mut futex_table = FUTEX_TABLE.lock();

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
