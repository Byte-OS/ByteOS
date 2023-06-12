#![no_std]

use bit_field::BitField;
use bitflags::bitflags;
use num_enum::TryFromPrimitive;

pub trait SignalOps {
    fn add_signal(&self, signum: usize);
    fn handle_signal(&self) -> usize;
    fn check_signal(&self) -> usize;
    fn sigmask(&self) -> usize;
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct SignalFlags: u64 {
        /// Hangup.
        const	SIGHUP		= 1 << ( 0);
        /// Interactive attention signal.
        const	SIGINT		= 1 << ( 1);
        /// Quit.
        const	SIGQUIT		= 1 << ( 2);
        /// Illegal instruction.
        const	SIGILL		= 1 << ( 3);
        /// Trace/breakpoint trap.
        const	SIGTRAP		= 1 << ( 4);
        /// IOT instruction, abort() on a PDP-11.
        const	SIGABRT		= 1 << ( 5);
        /// Bus error.
        const	SIGBUS		= 1 << ( 6);
        /// Erroneous arithmetic operation.
        const	SIGFPE		= 1 << ( 7);
        /// Killed.
        const	SIGKILL		= 1 << ( 8);
        /// User-defined signal 1.
        const	SIGUSR1		= 1 << ( 9);
        /// Invalid access to storage.
        const	SIGSEGV		= 1 << (10);
        /// User-defined signal 2.
        const	SIGUSR2		= 1 << (11);
        /// Broken pipe.
        const	SIGPIPE		= 1 << (12);
        /// Alarm clock.
        const	SIGALRM		= 1 << (13);
        /// Termination request.
        const	SIGTERM		= 1 << (14);
        const	SIGSTKFLT	= 1 << (15);
        /// Child terminated or stopped.
        const	SIGCHLD		= 1 << (16);
        /// Continue.
        const	SIGCONT		= 1 << (17);
        /// Stop, unblockable.
        const	SIGSTOP		= 1 << (18);
        /// Keyboard stop.
        const	SIGTSTP		= 1 << (19);
        /// Background read from control terminal.
        const	SIGTTIN		= 1 << (20);
        /// Background write to control terminal.
        const	SIGTTOU		= 1 << (21);
        /// Urgent data is available at a socket.
        const	SIGURG		= 1 << (22);
        /// CPU time limit exceeded.
        const	SIGXCPU		= 1 << (23);
        /// File size limit exceeded.
        const	SIGXFSZ		= 1 << (24);
        /// Virtual timer expired.
        const	SIGVTALRM	= 1 << (25);
        /// Profiling timer expired.
        const	SIGPROF		= 1 << (26);
        /// Window size change (4.3 BSD, Sun).
        const	SIGWINCH	= 1 << (27);
        /// I/O now possible (4.2 BSD).
        const	SIGIO		= 1 << (28);
        const   SIGPWR      = 1 << (29);
        /// Bad system call.
        const   SIGSYS      = 1 << (30);
        /* --- realtime signals for pthread --- */
        const   SIGTIMER    = 1 << (31);
        const   SIGCANCEL   = 1 << (32);
        const   SIGSYNCCALL = 1 << (33);
        /* --- other realtime signals --- */
        const   SIGRT_3     = 1 << (34);
        const   SIGRT_4     = 1 << (35);
        const   SIGRT_5     = 1 << (36);
        const   SIGRT_6     = 1 << (37);
        const   SIGRT_7     = 1 << (38);
        const   SIGRT_8     = 1 << (39);
        const   SIGRT_9     = 1 << (40);
        const   SIGRT_10    = 1 << (41);
        const   SIGRT_11    = 1 << (42);
        const   SIGRT_12    = 1 << (43);
        const   SIGRT_13    = 1 << (44);
        const   SIGRT_14    = 1 << (45);
        const   SIGRT_15    = 1 << (46);
        const   SIGRT_16    = 1 << (47);
        const   SIGRT_17    = 1 << (48);
        const   SIGRT_18    = 1 << (49);
        const   SIGRT_19    = 1 << (50);
        const   SIGRT_20    = 1 << (51);
        const   SIGRT_21    = 1 << (52);
        const   SIGRT_22    = 1 << (53);
        const   SIGRT_23    = 1 << (54);
        const   SIGRT_24    = 1 << (55);
        const   SIGRT_25    = 1 << (56);
        const   SIGRT_26    = 1 << (57);
        const   SIGRT_27    = 1 << (58);
        const   SIGRT_28    = 1 << (59);
        const   SIGRT_29    = 1 << (60);
        const   SIGRT_30    = 1 << (61);
        const   SIGRT_31    = 1 << (62);
        const   SIGRTMAX    = 1 << (63);

    }
}

impl SignalFlags {
    #[inline]
    pub fn from_usize(num: usize) -> SignalFlags {
        SignalFlags::from_bits_truncate(1 << (num - 1))
    }

    #[inline]
    pub fn num(&self) -> usize {
        let bits = self.bits();

        for i in 0..64 {
            if bits.get_bit(i) {
                return i + 1;
            }
        }
        0
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

    pub fn masked(&self, signum: usize) -> bool {
        (self.mask >> signum) & 1 == 0
    }
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
