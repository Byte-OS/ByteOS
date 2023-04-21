#![no_std]

extern crate alloc;

extern crate logging;

mod executor;
mod ops;
mod task;
pub mod thread;

pub use executor::*;
pub use futures::future::select;
pub use ops::*;
pub use task::*;

// tms_utime记录的是进程执行用户代码的时间.
// tms_stime记录的是进程执行内核代码的时间.
// tms_cutime记录的是子进程执行用户代码的时间.
// tms_ustime记录的是子进程执行内核代码的时间.
#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub struct TMS {
    pub utime: u64,
    pub stime: u64,
    pub cutime: u64,
    pub cstime: u64,
}
