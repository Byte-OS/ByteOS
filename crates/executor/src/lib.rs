#![no_std]

extern crate alloc;

mod executor;
mod ops;
pub mod task;
pub mod thread;

use core::task::Poll;
use core::{future::Future, pin::Pin, task::Context};

use alloc::boxed::Box;
pub use executor::*;
pub use ops::*;
pub use task::AsyncTask;

pub struct Select<A, B> {
    inner: Option<(A, B)>,
}

impl<A: Unpin, B: Unpin> Unpin for Select<A, B> {}

pub fn select<A, B>(future1: A, future2: B) -> Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    Select {
        inner: Some((future1, future2)),
    }
}

fn poll_unpin<A: Future + Unpin>(future: A, cx: &mut Context<'_>) -> Poll<A::Output> {
    Box::pin(future).as_mut().poll(cx)
}

pub enum Either<A, B> {
    /// First branch of the type
    Left(/* #[pin] */ A),
    /// Second branch of the type
    Right(/* #[pin] */ B),
}

impl<A, B> Future for Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    type Output = Either<(A::Output, B), (B::Output, A)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        /// When compiled with `-C opt-level=z`, this function will help the compiler eliminate the `None` branch, where
        /// `Option::unwrap` does not.
        #[inline(always)]
        fn unwrap_option<T>(value: Option<T>) -> T {
            match value {
                None => unreachable!(),
                Some(value) => value,
            }
        }

        let (a, b) = self.inner.as_mut().expect("cannot poll Select twice");

        if let Poll::Ready(val) = poll_unpin(a, cx) {
            return Poll::Ready(Either::Left((val, unwrap_option(self.inner.take()).1)));
        }

        if let Poll::Ready(val) = poll_unpin(b, cx) {
            return Poll::Ready(Either::Right((val, unwrap_option(self.inner.take()).0)));
        }

        Poll::Pending
    }
}
