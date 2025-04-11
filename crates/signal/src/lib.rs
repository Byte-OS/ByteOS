#![no_std]

use bit_field::BitField;
use bitflags::bitflags;

pub trait SignalOps {
    fn add_signal(&self, signum: usize);
    fn handle_signal(&self) -> usize;
    fn check_signal(&self) -> usize;
    fn sigmask(&self) -> usize;
}

macro_rules! bit {
    ($x:expr) => {
        1 << $x
    };
}

pub const REAL_TIME_SIGNAL_NUM: usize = 33;

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct SignalFlags: u64 {
        /// Hangup.
        const	SIGHUP		= bit!(0);
        /// Interactive attention signal.
        const	SIGINT		= bit!(1);
        /// Quit.
        const	SIGQUIT		= bit!(2);
        /// Illegal instruction.
        const	SIGILL		= bit!(3);
        /// Trace/breakpoint trap.
        const	SIGTRAP		= bit!(4);
        /// IOT instruction, abort() on a PDP-11.
        const	SIGABRT		= bit!(5);
        /// Bus error.
        const	SIGBUS		= bit!(6);
        /// Erroneous arithmetic operation.
        const	SIGFPE		= bit!(7);
        /// Killed.
        const	SIGKILL		= bit!(8);
        /// User-defined signal 1.
        const	SIGUSR1		= bit!( 9);
        /// Invalid access to storage.
        const	SIGSEGV		= bit!(10);
        /// User-defined signal 2.
        const	SIGUSR2		= bit!(11);
        /// Broken pipe.
        const	SIGPIPE		= bit!(12);
        /// Alarm clock.
        const	SIGALRM		= bit!(13);
        /// Termination request.
        const	SIGTERM		= bit!(14);
        const	SIGSTKFLT	= bit!(15);
        /// Child terminated or stopped.
        const	SIGCHLD		= bit!(16);
        /// Continue.
        const	SIGCONT		= bit!(17);
        /// Stop, unblockable.
        const	SIGSTOP		= bit!(18);
        /// Keyboard stop.
        const	SIGTSTP		= bit!(19);
        /// Background read from control terminal.
        const	SIGTTIN		= bit!(20);
        /// Background write to control terminal.
        const	SIGTTOU		= bit!(21);
        /// Urgent data is available at a socket.
        const	SIGURG		= bit!(22);
        /// CPU time limit exceeded.
        const	SIGXCPU		= bit!(23);
        /// File size limit exceeded.
        const	SIGXFSZ		= bit!(24);
        /// Virtual timer expired.
        const	SIGVTALRM	= bit!(25);
        /// Profiling timer expired.
        const	SIGPROF		= bit!(26);
        /// Window size change (4.3 BSD, Sun).
        const	SIGWINCH	= bit!(27);
        /// I/O now possible (4.2 BSD).
        const	SIGIO		= bit!(28);
        const   SIGPWR      = bit!(29);
        /// Bad system call.
        const   SIGSYS      = bit!(30);
        /* --- realtime signals for pthread --- */
        const   SIGTIMER    = bit!(31);
        const   SIGCANCEL   = bit!(32);
        const   SIGSYNCCALL = bit!(33);
        /* --- other realtime signals --- */
        const   SIGRT_3     = bit!(34);
        const   SIGRT_4     = bit!(35);
        const   SIGRT_5     = bit!(36);
        const   SIGRT_6     = bit!(37);
        const   SIGRT_7     = bit!(38);
        const   SIGRT_8     = bit!(39);
        const   SIGRT_9     = bit!(40);
        const   SIGRT_10    = bit!(41);
        const   SIGRT_11    = bit!(42);
        const   SIGRT_12    = bit!(43);
        const   SIGRT_13    = bit!(44);
        const   SIGRT_14    = bit!(45);
        const   SIGRT_15    = bit!(46);
        const   SIGRT_16    = bit!(47);
        const   SIGRT_17    = bit!(48);
        const   SIGRT_18    = bit!(49);
        const   SIGRT_19    = bit!(50);
        const   SIGRT_20    = bit!(51);
        const   SIGRT_21    = bit!(52);
        const   SIGRT_22    = bit!(53);
        const   SIGRT_23    = bit!(54);
        const   SIGRT_24    = bit!(55);
        const   SIGRT_25    = bit!(56);
        const   SIGRT_26    = bit!(57);
        const   SIGRT_27    = bit!(58);
        const   SIGRT_28    = bit!(59);
        const   SIGRT_29    = bit!(60);
        const   SIGRT_30    = bit!(61);
        const   SIGRT_31    = bit!(62);
        const   SIGRTMAX    = bit!(63);

    }
}

impl SignalFlags {
    pub const fn from_num(num: usize) -> SignalFlags {
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

    #[inline]
    pub fn is_real_time(&self) -> bool {
        self.bits() & 0xFFFFFFFE00000000 != 0
    }

    #[inline]
    pub fn real_time_index(&self) -> Option<usize> {
        self.is_real_time().then(|| self.num() - 32)
    }
}

#[derive(Debug, Clone, Copy)]
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
    pub mask: usize,
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
