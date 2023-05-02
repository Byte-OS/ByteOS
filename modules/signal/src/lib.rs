#![no_std]

use bitflags::bitflags;
use num_enum::TryFromPrimitive;

pub trait SignalOps {
    fn add_signal(&self, signum: usize);
    fn handle_signal(&self) -> usize;
    fn check_signal(&self) -> usize;
    fn sigmask(&self) -> usize;
}

bitflags! {
    #[derive(Debug)]
    pub struct SignalFlags: u32 {
        const SIGDEF = 1; // Default signal handling
        const SIGHUP = 1 << 1;
        const SIGINT = 1 << 2;
        const SIGQUIT = 1 << 3;
        const SIGILL = 1 << 4;
        const SIGTRAP = 1 << 5;
        const SIGABRT = 1 << 6;
        const SIGIOT = 1 << 7;
        const SIGBUS = 1 << 8;
        const SIGFPE = 1 << 9;
        const SIGKILL = 1 << 10;
        const SIGUSR1 = 1 << 11;
        const SIGSEGV = 1 << 12;
        const SIGUSR2 = 1 << 13;
        const SIGPIPE = 1 << 14;
        const SIGALRM = 1 << 15;
        const SIGTERM = 1 << 16;
        const SIGSTKFLT = 1 << 17;
        const SIGCHLD = 1 << 18;
        const SIGCONT = 1 << 19;
        const SIGSTOP = 1 << 20;
        const SIGTSTP = 1 << 21;
        const SIGTTIN = 1 << 22;
        const SIGTTOU = 1 << 23;
        const SIGURG = 1 << 24;
        const SIGXCPU = 1 << 25;
        const SIGXFSZ = 1 << 26;
        const SIGVTALRM = 1 << 27;
        const SIGPROF = 1 << 28;
        const SIGWINCH = 1 << 29;
        const SIGIO = 1 << 30;
        const SIGSYS = 1 << 31;
    }
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum SigMaskHow {
    Block,
    Unblock,
    Setmask,
}

impl SigMaskHow {
    pub fn from_usize(how: usize) -> Option<Self> {
        match how {
            0 => Some(SigMaskHow::Block),
            1 => Some(SigMaskHow::Unblock),
            2 => Some(SigMaskHow::Setmask),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SigProcMask {
    mask: usize,
}

impl SigProcMask {
    pub fn new() -> Self {
        Self { mask: 0 }
    }

    pub fn handle(&mut self, how: SigMaskHow, mask: &Self) {
        self.mask = match how {
            SigMaskHow::Block => self.mask | mask.mask,
            SigMaskHow::Unblock => self.mask & (!mask.mask),
            SigMaskHow::Setmask => mask.mask,
        }
    }
}

/// musl riscv Sigaction
/// void (*handler)(int);
//  unsigned long flags;
//  void (*restorer)(void);
//  unsigned mask[2];
//  void *unused;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    pub handler: usize,    // void     (*sa_handler)(int);
    pub flags: usize,      // int        sa_flags;
    pub restorer: usize,   // void     (*sa_restorer)(void);
    pub mask: SigProcMask, // sigset_t   sa_mask;
}

impl SigAction {
    pub fn new() -> Self {
        Self {
            handler: 0,
            mask: SigProcMask::new(),
            flags: 0,
            restorer: 0,
        }
    }
}

// sigset_t sa_mask 是一个信号集，在调用该信号捕捉函数之前，将需要block的信号加入这个sa_mask，
// 仅当信号捕捉函数正在执行时，才阻塞sa_mask中的信号，当从信号捕捉函数返回时进程的信号屏蔽字复位为原先值。
