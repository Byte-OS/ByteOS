#![no_std]

use libc_types::types::SigSet;

pub trait SignalOps {
    fn add_signal(&self, signum: usize);
    fn handle_signal(&self) -> usize;
    fn check_signal(&self) -> usize;
    fn sigmask(&self) -> usize;
}

// musl riscv Sigaction
// struct Sigaction {
//     void (*handler)(int);
//     unsigned long flags;
//     void (*restorer)(void);
//     unsigned mask[2];
//     void *unused;
// }

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    pub handler: usize,  // void     (*sa_handler)(int);
    pub flags: usize,    // int        sa_flags;
    pub restorer: usize, // void     (*sa_restorer)(void);
    pub mask: SigSet,    // sigset_t   sa_mask;
}

impl SigAction {
    pub fn new() -> Self {
        Self {
            handler: 0,
            mask: SigSet::empty(),
            flags: 0,
            restorer: 0,
        }
    }
}

// sigset_t sa_mask 是一个信号集，在调用该信号捕捉函数之前，将需要block的信号加入这个sa_mask，
// 仅当信号捕捉函数正在执行时，才阻塞sa_mask中的信号，当从信号捕捉函数返回时进程的信号屏蔽字复位为原先值。
