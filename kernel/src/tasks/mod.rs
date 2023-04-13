use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use arch::get_time_ms;
use executor::{Executor, KernelTask};

use self::initproc::initproc;

mod initproc;

pub struct NextTick(usize);

impl Future for NextTick {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let curr = get_time_ms();
        if curr < self.0 {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub fn init() {
    let mut exec = Executor::new();
    exec.spawn(KernelTask::new(initproc()));
    // exec.spawn()
    exec.run();
}
