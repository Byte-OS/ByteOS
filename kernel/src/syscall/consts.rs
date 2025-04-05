//! allow dead code in the file
#![allow(dead_code)]

use core::cmp::Ordering;
use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use core::ops::Add;

use bitflags::bitflags;
use cfg_if::cfg_if;
use fs::VfsError;
use num_derive::FromPrimitive;
use polyhal::VirtAddr;
use polyhal::{MappingFlags, Time};
use polyhal_trap::trapframe::TrapFrame;
use signal::SigProcMask;
use syscalls::Errno;

pub fn from_vfs(vfs_error: VfsError) -> Errno {
    match vfs_error {
        VfsError::NotLinkFile => Errno::EBADF,
        VfsError::NotDir => Errno::ENOTDIR,
        VfsError::NotFile => Errno::EBADF,
        VfsError::NotSupported => Errno::EPERM,
        VfsError::FileNotFound => Errno::ENOENT,
        VfsError::AlreadyExists => Errno::EEXIST,
        VfsError::InvalidData => Errno::EIO,
        VfsError::DirectoryNotEmpty => Errno::ENOTEMPTY,
        VfsError::InvalidInput => Errno::EINVAL,
        VfsError::StorageFull => Errno::EIO,
        VfsError::UnexpectedEof => Errno::EIO,
        VfsError::WriteZero => Errno::EIO,
        VfsError::Io => Errno::EIO,
        VfsError::Blocking => Errno::EAGAIN,
        VfsError::NoMountedPoint => Errno::ENOENT,
        VfsError::NotAPipe => Errno::EPIPE,
        VfsError::NotWriteable => Errno::EBADF,
    }
}

pub const AT_CWD: usize = -100 as isize as usize;

pub struct UTSname {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

bitflags! {
    // MAP Flags
    #[derive(Debug)]
    pub struct MapFlags: u32 {
        const MAP_SHARED          =    0x01;
        const MAP_PRIVATE         =    0x02;
        const MAP_SHARED_VALIDATE =    0x03;
        const MAP_TYPE            =    0x0f;
        const MAP_FIXED           =    0x10;
        const MAP_ANONYMOUS       =    0x20;
        const MAP_NORESERVE       =    0x4000;
        const MAP_GROWSDOWN       =    0x0100;
        const MAP_DENYWRITE       =    0x0800;
        const MAP_EXECUTABLE      =    0x1000;
        const MAP_LOCKED          =    0x2000;
        const MAP_POPULATE        =    0x8000;
        const MAP_NONBLOCK        =    0x10000;
        const MAP_STACK           =    0x20000;
        const MAP_HUGETLB         =    0x40000;
        const MAP_SYNC            =    0x80000;
        const MAP_FIXED_NOREPLACE =    0x100000;
        const MAP_FILE            =    0;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct MmapProt: u32 {
        const PROT_READ = 1 << 0;
        const PROT_WRITE = 1 << 1;
        const PROT_EXEC = 1 << 2;
    }

    #[derive(Debug)]
    pub struct MSyncFlags: u32 {
        const ASYNC = 1 << 0;
        const INVALIDATE = 1 << 1;
        const SYNC = 1 << 2;
    }

    #[derive(Debug)]
    pub struct ProtFlags: u32 {
        const PROT_NONE = 0;
        const PROT_READ = 1;
        const PROT_WRITE = 2;
        const PROT_EXEC = 4;
    }

    #[derive(Debug)]
    pub struct CloneFlags: usize {
        const CSIGNAL		        = 0x000000ff;
        const CLONE_VM	            = 0x00000100;
        const CLONE_FS	            = 0x00000200;
        const CLONE_FILES	        = 0x00000400;
        const CLONE_SIGHAND	        = 0x00000800;
        const CLONE_PIDFD	        = 0x00001000;
        const CLONE_PTRACE	        = 0x00002000;
        const CLONE_VFORK	        = 0x00004000;
        const CLONE_PARENT	        = 0x00008000;
        const CLONE_THREAD	        = 0x00010000;
        const CLONE_NEWNS	        = 0x00020000;
        const CLONE_SYSVSEM	        = 0x00040000;
        const CLONE_SETTLS	        = 0x00080000;
        const CLONE_PARENT_SETTID	= 0x00100000;
        const CLONE_CHILD_CLEARTID	= 0x00200000;
        const CLONE_DETACHED	    = 0x00400000;
        const CLONE_UNTRACED	    = 0x00800000;
        const CLONE_CHILD_SETTID	= 0x01000000;
        const CLONE_NEWCGROUP	    = 0x02000000;
        const CLONE_NEWUTS	        = 0x04000000;
        const CLONE_NEWIPC	        = 0x08000000;
        const CLONE_NEWUSER	        = 0x10000000;
        const CLONE_NEWPID	        = 0x20000000;
        const CLONE_NEWNET	        = 0x40000000;
        const CLONE_IO	            = 0x80000000;
    }
}

impl Into<MappingFlags> for MmapProt {
    fn into(self) -> MappingFlags {
        let mut res = MappingFlags::empty();
        if self.contains(Self::PROT_READ) {
            res |= MappingFlags::R;
        }
        if self.contains(Self::PROT_WRITE) {
            res |= MappingFlags::W;
        }
        if self.contains(Self::PROT_EXEC) {
            res |= MappingFlags::X;
        }
        res
    }
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

pub mod elf {
    pub const AT_NULL: usize = 0;
    pub const AT_IGNORE: usize = 1;
    pub const AT_EXECFD: usize = 2;
    pub const AT_PHDR: usize = 3;
    pub const AT_PHENT: usize = 4;
    pub const AT_PHNUM: usize = 5;
    pub const AT_PAGESZ: usize = 6;
    pub const AT_BASE: usize = 7;
    pub const AT_FLAGS: usize = 8;
    pub const AT_ENTRY: usize = 9;
    pub const AT_NOTELF: usize = 10;
    pub const AT_UID: usize = 11;
    pub const AT_EUID: usize = 12;
    pub const AT_GID: usize = 13;
    pub const AT_EGID: usize = 14;
    pub const AT_PLATFORM: usize = 15;
    pub const AT_HWCAP: usize = 16;
    pub const AT_CLKTCK: usize = 17;
    pub const AT_FPUCW: usize = 18;
    pub const AT_DCACHEBSIZE: usize = 19;
    pub const AT_ICACHEBSIZE: usize = 20;
    pub const AT_UCACHEBSIZE: usize = 21;
    pub const AT_IGNOREPPC: usize = 22;
    pub const AT_SECURE: usize = 23;
    pub const AT_BASE_PLATFORM: usize = 24;
    pub const AT_RANDOM: usize = 25;
    pub const AT_HWCAP2: usize = 26;

    pub const AT_EXECFN: usize = 31;
    pub const AT_SYSINFO: usize = 32;
    pub const AT_SYSINFO_EHDR: usize = 33;
}

#[allow(unused)]
pub mod fcntl_cmd {
    /// dup
    pub const DUPFD: usize = 0;
    /// get close_on_exec
    pub const GETFD: usize = 1;
    /// set/clear close_on_exec
    pub const SETFD: usize = 2;
    /// get file->f_flags
    pub const GETFL: usize = 3;
    /// set file->f_flags
    pub const SETFL: usize = 4;
    /// Get record locking info.
    pub const GETLK: usize = 5;
    /// Set record locking info (non-blocking).
    pub const SETLK: usize = 6;
    /// Set record locking info (blocking).
    pub const SETLKW: usize = 7;
    /// like F_DUPFD, but additionally set the close-on-exec flag
    pub const DUPFD_CLOEXEC: usize = 0x406;
}

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

#[repr(usize)]
#[derive(Debug, Clone, FromPrimitive)]
#[allow(non_camel_case_types)]
pub enum ArchPrctlCode {
    ARCH_SET_GS = 0x1001,
    ARCH_SET_FS = 0x1002,
    ARCH_GET_FS = 0x1003,
    ARCH_GET_GS = 0x1004,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Rlimit {
    pub curr: usize,
    pub max: usize,
}

pub const RLIMIT_NOFILE: usize = 7;

bitflags! {
    #[derive(Debug, Clone)]
    pub struct SignalStackFlags : u32 {
        const ONSTACK = 1;
        const DISABLE = 2;
        const AUTODISARM = 0x80000000;
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SignalStack {
    pub sp: usize,
    pub flags: SignalStackFlags,
    pub size: usize,
}

cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct SignalUserContext {
            pub flags: usize,          // 0
            pub link: usize,           // 1
            pub stack: SignalStack,    // 2
            pub gregs: [usize; 32],
            pub sig_mask: SigProcMask, // sigmask
            pub _pad: [u64; 16],       // sigmask extend
            pub __fpregs_mem: [u64; 64]
        }

        impl SignalUserContext {
            pub fn pc(&self) -> usize {
                self.gregs[16]
            }

            pub fn set_pc(&mut self, v: usize) {
                self.gregs[16] = v;
            }

            pub fn store_ctx(&mut self, ctx: &TrapFrame) {
                self.gregs[0] = ctx.r8;
                self.gregs[1] = ctx.r9;
                self.gregs[2] = ctx.r10;
                self.gregs[3] = ctx.r11;
                self.gregs[4] = ctx.r12;
                self.gregs[5] = ctx.r13;
                self.gregs[6] = ctx.r14;
                self.gregs[7] = ctx.r15;
                self.gregs[8] = ctx.rdi;
                self.gregs[9] = ctx.rsi;
                self.gregs[10] = ctx.rbp;
                self.gregs[11] = ctx.rbx;
                self.gregs[12] = ctx.rdx;
                self.gregs[13] = ctx.rax;
                self.gregs[14] = ctx.rcx;
                self.gregs[15] = ctx.rsp;
                self.gregs[16] = ctx.rip;
            }

            pub fn restore_ctx(&self, ctx: &mut TrapFrame) {
                ctx.r8  = self.gregs[0];
                ctx.r9  = self.gregs[1];
                ctx.r10 = self.gregs[2];
                ctx.r11 = self.gregs[3];
                ctx.r12 = self.gregs[4];
                ctx.r13 = self.gregs[5];
                ctx.r14 = self.gregs[6];
                ctx.r15 = self.gregs[7];
                ctx.rdi = self.gregs[8];
                ctx.rsi = self.gregs[9];
                ctx.rbp = self.gregs[10];
                ctx.rbx = self.gregs[11];
                ctx.rdx = self.gregs[12];
                ctx.rax = self.gregs[13];
                ctx.rcx = self.gregs[14];
                ctx.rsp = self.gregs[15];
                ctx.rip = self.gregs[16];
            }
        }
    } else if #[cfg(target_arch = "riscv64")] {
        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct SignalUserContext {
            pub flags: usize,          // 0
            pub link: usize,           // 1
            pub stack: SignalStack,    // 2
            pub sig_mask: SigProcMask, // 5
            pub _pad: [u64; 16],       // mask
            // pub context: Context,       // pc offset = 22 - 6=16
            pub gregs: [usize; 32],
            pub fpstate: [usize; 66],
        }

        impl SignalUserContext {
            pub fn pc(&self) -> usize {
                self.gregs[0]
            }

            pub fn set_pc(&mut self, v: usize) {
                self.gregs[0] = v;
            }

            pub fn store_ctx(&mut self, ctx: &TrapFrame) {
                self.gregs = ctx.x;
            }

            pub fn restore_ctx(&self, ctx: &mut TrapFrame) {
                ctx.x = self.gregs;
            }
        }
    } else if #[cfg(target_arch = "aarch64")] {
        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct SignalUserContext {
            pub flags: usize,          // 0
            pub link: usize,           // 1
            pub stack: SignalStack,    // 2
            pub sig_mask: SigProcMask, // 5
            pub _pad: [u64; 16],       // mask
            pub fault_address: usize,
            pub regs: [usize; 31],
            pub sp: usize,
            pub pc: usize,
            pub pstate: usize,
            pub __reserved: usize,
        }

        impl SignalUserContext {
            pub fn pc(&self) -> usize {
                self.pc
            }

            pub fn set_pc(&mut self, v: usize) {
                self.pc = v;
            }

            pub fn store_ctx(&mut self, ctx: &TrapFrame) {
                self.regs = ctx.regs;
            }

            pub fn restore_ctx(&self, ctx: &mut TrapFrame) {
                ctx.regs = self.regs;
            }
        }
    } else if #[cfg(target_arch = "loongarch64")] {
        #[repr(C)]
        #[derive(Debug, Clone)]
        pub struct SignalUserContext {
            pub flags: usize,          // 0
            pub link: usize,           // 1
            pub stack: SignalStack,    // 2
            pub sig_mask: SigProcMask, // 5
            pub _pad: [u64; 2],       // mask
            pub pc: usize,
            pub gregs: [usize; 32],
            pub gflags: u32,
            pub fcsr: u32,
            pub scr: [usize; 4],
            pub fregs: [usize; 32],        // _extcontext
            pub _reserved: [usize; 512],
        }

        impl SignalUserContext {
            pub fn pc(&self) -> usize {
                self.pc
            }

            pub fn set_pc(&mut self, v: usize) {
                self.pc = v;
            }

            pub fn store_ctx(&mut self, ctx: &TrapFrame) {
                self.gregs = ctx.regs;
            }

            pub fn restore_ctx(&self, ctx: &mut TrapFrame) {
                ctx.regs = self.gregs;
            }
        }
    }
}

#[repr(C)]
pub struct Rusage {
    pub ru_utime: TimeVal,
    pub ru_stime: TimeVal,
    pub ru_maxrss: i64,
    pub ru_ixrss: i64,
    pub ru_idrss: i64,
    pub ru_isrss: i64,
    pub ru_minflt: i64,
    pub ru_majflt: i64,
    pub ru_nswap: i64,
    pub ru_inblock: i64,
    pub ru_oublock: i64,
    pub ru_msgsnd: i64,
    pub ru_msgrcv: i64,
    pub ru_nsignals: i64,
    pub ru_nvcsw: i64,
    pub ru_nivcsw: i64,
}

#[derive(Clone, Copy)]
pub struct UserRef<T> {
    addr: VirtAddr,
    r#type: PhantomData<T>,
}

impl<T> From<usize> for UserRef<T> {
    fn from(value: usize) -> Self {
        Self {
            addr: value.into(),
            r#type: PhantomData,
        }
    }
}

impl<T> From<VirtAddr> for UserRef<T> {
    fn from(value: VirtAddr) -> Self {
        Self {
            addr: value,
            r#type: PhantomData,
        }
    }
}

impl<T> Into<usize> for UserRef<T> {
    fn into(self) -> usize {
        self.addr.raw()
    }
}

impl<T> UserRef<T> {
    #[inline]
    pub fn addr(&self) -> usize {
        self.addr.raw()
    }
    #[inline]
    pub fn get_ref(&self) -> &'static T {
        self.addr.get_ref::<T>()
    }

    #[inline]
    pub fn get_mut(&self) -> &'static mut T {
        self.addr.get_mut_ref::<T>()
    }

    #[inline]
    pub fn slice_mut_with_len(&self, len: usize) -> &'static mut [T] {
        self.addr.slice_mut_with_len(len)
    }

    #[inline]
    pub fn slice_until_valid(&self, is_valid: fn(T) -> bool) -> &'static mut [T] {
        if self.addr.raw() == 0 {
            return &mut [];
        }
        self.addr.slice_until(is_valid)
    }

    #[inline]
    pub fn get_cstr(&self) -> Result<&str, core::str::Utf8Error> {
        self.addr.get_cstr().to_str()
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.addr.raw() != 0
    }
}

impl<T> Display for UserRef<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}({:#x})",
            core::any::type_name::<T>(),
            self.addr.raw()
        ))
    }
}

impl<T> Debug for UserRef<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}({:#x})",
            core::any::type_name::<T>(),
            self.addr.raw()
        ))
    }
}

pub fn current_nsec() -> usize {
    // devices::RTC_DEVICES.lock()[0].read() as usize
    // arch::time_to_usec(arch::get_time()) * 1000
    Time::now().to_nsec()
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeVal {
    pub sec: usize,  /* 秒 */
    pub usec: usize, /* 微秒, 范围在0~999999 */
}

impl TimeVal {
    pub fn now() -> Self {
        let ns = current_nsec();
        Self {
            sec: ns / 1_000_000_000,
            usec: (ns % 1_000_000_000) / 1000,
        }
    }
}

impl Add for TimeVal {
    type Output = TimeVal;

    fn add(self, rhs: Self) -> Self::Output {
        let nsec = self.usec + rhs.usec;
        Self {
            sec: self.sec + rhs.sec + nsec / 1_000_000_000,
            usec: nsec % 1_000_000_000,
        }
    }
}

impl PartialOrd for TimeVal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.sec > other.sec {
            Some(Ordering::Greater)
        } else if self.sec < other.sec {
            Some(Ordering::Less)
        } else {
            if self.usec > other.usec {
                Some(Ordering::Greater)
            } else if self.usec < other.usec {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Equal)
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, Copy)]
pub struct ITimerVal {
    pub interval: TimeVal,
    pub value: TimeVal,
}
