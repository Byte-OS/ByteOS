use num_derive::FromPrimitive;

pub const AT_CWD: usize = -100 as isize as usize;

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, FromPrimitive)]
pub enum FcntlCmd {
    /// dup
    DUPFD = 0,
    /// get close_on_exec
    GETFD = 1,
    /// set/clear close_on_exec
    SETFD = 2,
    /// get file->f_flags
    GETFL = 3,
    /// set file->f_flags
    SETFL = 4,
    /// Get record locking info.
    GETLK = 5,
    /// Set record locking info (non-blocking).
    SETLK = 6,
    /// Set record locking info (blocking).
    SETLKW = 7,
    /// like F_DUPFD, but additionally set the close-on-exec flag
    DUPFDCLOEXEC = 0x406,
}

#[derive(Debug, FromPrimitive)]
#[repr(usize)]
pub enum FutexFlags {
    Wait = 0,
    Wake = 1,
    Fd = 2,
    Requeue = 3,
    CmpRequeue = 4,
    WakeOp = 5,
    LockPi = 6,
    UnlockPi = 7,
    TrylockPi = 8,
    WaitBitset = 9,
}

#[repr(C)]
#[derive(Clone)]
pub struct IoVec {
    pub base: usize,
    pub len: usize,
}
