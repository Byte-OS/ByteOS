use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use alloc::sync::Arc;
use sync::Mutex;

use crate::FutexTable;

pub struct Yield(bool);

impl Yield {
    pub const fn new() -> Self {
        Self(false)
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0 {
            true => Poll::Ready(()),
            false => {
                self.0 = true;
                Poll::Pending
            }
        }
    }
}

pub async fn yield_now() {
    Yield::new().await;
}

#[crate_interface::def_interface]
pub trait FutexOps {
    fn futex_wake(
        task: Arc<Mutex<FutexTable>>,
        uaddr: usize,
        wake_count: usize,
    ) -> usize;
}
