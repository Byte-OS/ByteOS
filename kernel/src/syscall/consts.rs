//! allow dead code in the file
#![allow(dead_code)]

use core::fmt::{Debug, Display};
use core::marker::PhantomData;

use arch::{MappingFlags, VirtAddr};
use bitflags::bitflags;
use fs::VfsError;
use hal::TimeVal;
use num_derive::FromPrimitive;
use signal::SigProcMask;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxError {
    /// Operation not permitted
    EPERM = 1,
    /// No such file or directory
    ENOENT = 2,
    /// No such process
    ESRCH = 3,
    /// Interrupted system call
    EINTR = 4,
    /// I/O error
    EIO = 5,
    /// No such device or address
    ENXIO = 6,
    /// Argument list too long
    E2BIG = 7,
    /// Exec format error
    ENOEXEC = 8,
    /// Bad file number
    EBADF = 9,
    /// No child processes
    ECHILD = 10,
    /// Try again
    EAGAIN = 11,
    /// Out of memory
    ENOMEM = 12,
    /// Permission denied
    EACCES = 13,
    /// Bad address
    EFAULT = 14,
    /// Block device required
    ENOTBLK = 15,
    /// Device or resource busy
    EBUSY = 16,
    /// File exists
    EEXIST = 17,
    /// Cross-device link
    EXDEV = 18,
    /// No such device
    ENODEV = 19,
    /// Not a directory
    ENOTDIR = 20,
    /// Is a directory
    EISDIR = 21,
    /// Invalid argument
    EINVAL = 22,
    /// File table overflow
    ENFILE = 23,
    /// Too many open files
    EMFILE = 24,
    /// Not a typewriter
    ENOTTY = 25,
    /// Text file busy
    ETXTBSY = 26,
    /// File too large
    EFBIG = 27,
    /// No space left on device
    ENOSPC = 28,
    /// Illegal seek
    ESPIPE = 29,
    /// Read-only file system
    EROFS = 30,
    /// Too many links
    EMLINK = 31,
    /// Broken pipe
    EPIPE = 32,
    /// Math argument out of domain of func
    EDOM = 33,
    /// Math result not representable
    ERANGE = 34,
    /// Resource deadlock would occur
    EDEADLK = 35,
    /// File name too long
    ENAMETOOLONG = 36,
    /// No record locks available
    ENOLCK = 37,
    /// Invalid system call number
    ENOSYS = 38,
    /// Directory not empty
    ENOTEMPTY = 39,
    /// Address family not supported
    EAFNOSUPPORT = 97,
    /// Transport endpoint is not connected
    ENOTCONN = 107,
    /// Connection time out
    ETIMEDOUT = 100,
    /// Connection refused
    ECONNREFUSED = 111,
    /// Aleady used
    EALREADY = 114,
}

impl LinuxError {
    pub const fn as_str(&self) -> &'static str {
        use self::LinuxError::*;
        match self {
            EPERM => "Operation not permitted",
            ENOENT => "No such file or directory",
            ESRCH => "No such process",
            EINTR => "Interrupted system call",
            EIO => "I/O error",
            ENXIO => "No such device or address",
            E2BIG => "Argument list too long",
            ENOEXEC => "Exec format error",
            EBADF => "Bad file number",
            ECHILD => "No child processes",
            EAGAIN => "Try again",
            ENOMEM => "Out of memory",
            EACCES => "Permission denied",
            EFAULT => "Bad address",
            ENOTBLK => "Block device required",
            EBUSY => "Device or resource busy",
            EEXIST => "File exists",
            EXDEV => "Cross-device link",
            ENODEV => "No such device",
            ENOTDIR => "Not a directory",
            EISDIR => "Is a directory",
            EINVAL => "Invalid argument",
            ENFILE => "File table overflow",
            EMFILE => "Too many open files",
            ENOTTY => "Not a typewriter",
            ETXTBSY => "Text file busy",
            EFBIG => "File too large",
            ENOSPC => "No space left on device",
            ESPIPE => "Illegal seek",
            EROFS => "Read-only file system",
            EMLINK => "Too many links",
            EPIPE => "Broken pipe",
            EDOM => "Math argument out of domain of func",
            ERANGE => "Math result not representable",
            EDEADLK => "Resource deadlock would occur",
            ENAMETOOLONG => "File name too long",
            ENOLCK => "No record locks available",
            ENOSYS => "Invalid system call number",
            ENOTEMPTY => "Directory not empty",
            EAFNOSUPPORT => "Address family not supported",
            ENOTCONN => "Transport endpoint is not connected",
            ETIMEDOUT => "Connection time out",
            ECONNREFUSED => "Connection refused",
            EALREADY => "Port already used",
        }
    }

    pub const fn code(self) -> isize {
        self as isize
    }
}

pub fn from_vfs(vfs_error: VfsError) -> LinuxError {
    match vfs_error {
        VfsError::NotLinkFile => LinuxError::EBADF,
        VfsError::NotDir => LinuxError::ENOTDIR,
        VfsError::NotFile => LinuxError::EBADF,
        VfsError::NotSupported => LinuxError::EPERM,
        VfsError::FileNotFound => LinuxError::ENOENT,
        VfsError::AlreadyExists => LinuxError::EEXIST,
        VfsError::InvalidData => LinuxError::EIO,
        VfsError::DirectoryNotEmpty => LinuxError::ENOTEMPTY,
        VfsError::InvalidInput => LinuxError::EINVAL,
        VfsError::StorageFull => LinuxError::EIO,
        VfsError::UnexpectedEof => LinuxError::EIO,
        VfsError::WriteZero => LinuxError::EIO,
        VfsError::Io => LinuxError::EIO,
        VfsError::Blocking => LinuxError::EAGAIN,
        VfsError::NoMountedPoint => LinuxError::ENOENT,
        VfsError::NotAPipe => LinuxError::EPIPE,
        VfsError::NotWriteable => LinuxError::EBADF,
    }
}

// 中断调用列表
cfg_if::cfg_if! {
    if #[cfg(any(target_arch = "riscv", target_arch = "aarch64", target_arch = "loongarch64"))] {
        pub const SYS_GETCWD: usize = 17;
        pub const SYS_EPOLL_CREATE: usize = 20;
        pub const SYS_EPOLL_CTL: usize = 21;
        pub const SYS_EPOLL_WAIT: usize = 22;
        pub const SYS_DUP: usize = 23;
        pub const SYS_DUP3: usize = 24;
        pub const SYS_FCNTL: usize = 25;
        pub const SYS_IOCTL: usize = 29;
        pub const SYS_MKDIRAT: usize = 34;
        pub const SYS_UNLINKAT: usize = 35;
        pub const SYS_UMOUNT2: usize = 39;
        pub const SYS_MOUNT: usize = 40;
        pub const SYS_STATFS: usize = 43;
        pub const SYS_FTRUNCATE: usize = 46;
        pub const SYS_FACCESSAT: usize = 48;
        pub const SYS_CHDIR: usize = 49;
        pub const SYS_OPENAT: usize = 56;
        pub const SYS_CLOSE: usize = 57;
        pub const SYS_PIPE2: usize = 59;
        pub const SYS_GETDENTS: usize = 61;
        pub const SYS_LSEEK: usize = 62;
        pub const SYS_READ: usize = 63;
        pub const SYS_WRITE: usize = 64;
        pub const SYS_READV: usize = 65;
        pub const SYS_WRITEV: usize = 66;
        pub const SYS_PREAD: usize = 67;
        pub const SYS_PWRITE: usize = 68;
        pub const SYS_SENDFILE: usize = 71;
        pub const SYS_PSELECT: usize = 72;
        pub const SYS_PPOLL: usize = 73;
        pub const SYS_READLINKAT: usize = 78;
        pub const SYS_FSTATAT: usize = 79;
        pub const SYS_FSTAT: usize = 80;
        pub const SYS_FSYNC: usize = 82;
        pub const SYS_UTIMEAT: usize = 88;
        pub const SYS_EXIT: usize = 93;
        pub const SYS_EXIT_GROUP: usize = 94;
        pub const SYS_SET_TID_ADDRESS: usize = 96;
        pub const SYS_FUTEX: usize = 98;
        pub const SYS_SET_ROBUST_LIST: usize = 99;
        pub const SYS_GET_ROBUST_LIST: usize = 100;
        pub const SYS_NANOSLEEP: usize = 101;
        pub const SYS_SETITIMER: usize = 103;
        pub const SYS_GETTIME: usize = 113;
        pub const SYS_CLOCK_GETRES: usize = 114;
        pub const SYS_CLOCK_NANOSLEEP: usize = 115;
        pub const SYS_KLOGCTL: usize = 116;
        pub const SYS_SCHED_SETSCHEDULER: usize = 119;
        pub const SYS_SCHED_GETSCHEDULER: usize = 120;
        pub const SYS_SCHED_GETPARAM: usize = 121;
        pub const SYS_SCHED_SETAFFINITY: usize = 122;
        pub const SYS_SCHED_GETAFFINITY: usize = 123;
        pub const SYS_SCHED_YIELD: usize = 124;
        pub const SYS_KILL: usize = 129;
        pub const SYS_TKILL: usize = 130;
        pub const SYS_TGKILL: usize = 131;
        pub const SYS_SIGSUSPEND: usize = 133;
        pub const SYS_SIGACTION: usize = 134;
        pub const SYS_SIGPROCMASK: usize = 135;
        pub const SYS_SIGTIMEDWAIT: usize = 137;
        pub const SYS_SIGRETURN: usize = 139;
        pub const SYS_TIMES: usize = 153;
        pub const SYS_SETPGID: usize = 154;
        pub const SYS_GETPGID: usize = 155;
        pub const SYS_SETSID: usize = 157;
        pub const SYS_UNAME: usize = 160;
        pub const SYS_GETRUSAGE: usize = 165;
        pub const SYS_GETTIMEOFDAY: usize = 169;
        pub const SYS_GETPID: usize = 172;
        pub const SYS_GETPPID: usize = 173;
        pub const SYS_GETUID: usize = 174;
        pub const SYS_GETEUID: usize = 175;
        pub const SYS_GETGID: usize = 176;
        pub const SYS_GETEGID: usize = 177;
        pub const SYS_GETTID: usize = 178;
        pub const SYS_SYSINFO: usize = 179;
        pub const SYS_SHMGET: usize = 194;
        pub const SYS_SHMCTL: usize = 195;
        pub const SYS_SHMAT: usize = 196;
        pub const SYS_SOCKET: usize = 198;
        pub const SYS_SOCKETPAIR: usize = 199;
        pub const SYS_BIND: usize = 200;
        pub const SYS_LISTEN: usize = 201;
        pub const SYS_ACCEPT: usize = 202;
        pub const SYS_CONNECT: usize = 203;
        pub const SYS_GETSOCKNAME: usize = 204;
        pub const SYS_GETPEERNAME: usize = 205;
        pub const SYS_SENDTO: usize = 206;
        pub const SYS_RECVFROM: usize = 207;
        pub const SYS_SETSOCKOPT: usize = 208;
        pub const SYS_GETSOCKOPT: usize = 209;
        pub const SYS_SHUTDOWN: usize = 210;
        pub const SYS_BRK: usize = 214;
        pub const SYS_CLONE: usize = 220;
        pub const SYS_EXECVE: usize = 221;
        pub const SYS_MMAP: usize = 222;
        pub const SYS_MPROTECT: usize = 226;
        pub const SYS_MSYNC: usize = 227;
        pub const SYS_MUNMAP: usize = 215;
        pub const SYS_ACCEPT4: usize = 242;
        pub const SYS_WAIT4: usize = 260;
        pub const SYS_PRLIMIT64: usize = 261;
        pub const SYS_GETRANDOM: usize = 278;
        pub const SYS_COPY_FILE_RANGE: usize = 285;
        pub const SYS_FACCESSAT2: usize = 439;
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
        let mut res = MappingFlags::None;
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

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Rlimit {
    pub curr: usize,
    pub max: usize,
}

pub const RLIMIT_NOFILE: usize = 7;

bitflags! {
    #[derive(Clone)]
    pub struct SignalStackFlags : u32 {
        const ONSTACK = 1;
        const DISABLE = 2;
        const AUTODISARM = 0x80000000;
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct SignalStack {
    pub sp: usize,
    pub flags: SignalStackFlags,
    pub size: usize,
}

#[repr(C)]
#[derive(Clone)]
pub struct SignalUserContext {
    pub flags: usize,          // 0
    pub link: usize,           // 1
    pub stack: SignalStack,    // 2
    pub sig_mask: SigProcMask, // 5
    pub _pad: [u64; 16],
    // pub context: Context,       // pc offset = 22 - 6=16
    pub pc: usize,
    pub reserved: [usize; 17],
    pub fpstate: [usize; 66],
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
        self.addr.addr()
    }
}

impl<T> UserRef<T> {
    #[inline]
    pub fn addr(&self) -> usize {
        self.addr.addr()
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
        self.addr.slice_until(is_valid)
    }

    #[inline]
    pub fn get_cstr(&self) -> Result<&str, core::str::Utf8Error> {
        self.addr.get_cstr().to_str()
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.addr.addr() != 0
    }
}

impl<T> Display for UserRef<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}({:#x})",
            core::any::type_name::<T>(),
            self.addr.addr()
        ))
    }
}

impl<T> Debug for UserRef<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "{}({:#x})",
            core::any::type_name::<T>(),
            self.addr.addr()
        ))
    }
}
