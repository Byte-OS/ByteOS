pub mod consts;
mod fd;
mod func;
mod mm;
mod signal;
mod sys;
mod task;
mod time;

pub use task::exec_with_process;

use log::warn;

use self::{
    consts::{
        LinuxError, SYS_BRK, SYS_CHDIR, SYS_CLONE, SYS_CLOSE, SYS_DUP, SYS_DUP3, SYS_EXECVE,
        SYS_EXIT, SYS_FCNTL, SYS_FSTAT, SYS_FSTATAT, SYS_FUTEX, SYS_GETCWD, SYS_GETDENTS,
        SYS_GETEGID, SYS_GETEUID, SYS_GETGID, SYS_GETPGID, SYS_GETPID, SYS_GETPPID, SYS_GETTID,
        SYS_GETTIME, SYS_GETTIMEOFDAY, SYS_GETUID, SYS_IOCTL, SYS_LSEEK, SYS_MKDIRAT, SYS_MMAP,
        SYS_MOUNT, SYS_MPROTECT, SYS_MUNMAP, SYS_NANOSLEEP, SYS_OPENAT, SYS_PIPE2, SYS_PREAD,
        SYS_PRLIMIT64, SYS_READ, SYS_READV, SYS_SCHED_YIELD, SYS_SET_TID_ADDRESS, SYS_SIGACTION,
        SYS_SIGPROCMASK, SYS_SIGTIMEDWAIT, SYS_STATFS, SYS_TIMES, SYS_UMOUNT2, SYS_UNAME,
        SYS_UNLINKAT, SYS_UTIMEAT, SYS_WAIT4, SYS_WRITE, SYS_WRITEV,
    },
    fd::{
        sys_close, sys_dup, sys_dup3, sys_fcntl, sys_fstat, sys_fstatat, sys_getdents64, sys_ioctl,
        sys_lseek, sys_mkdir_at, sys_mount, sys_openat, sys_pipe2, sys_pread, sys_read, sys_readv,
        sys_statfs, sys_umount2, sys_unlinkat, sys_utimensat, sys_write, sys_writev,
    },
    mm::{sys_brk, sys_mmap, sys_mprotect, sys_munmap},
    signal::{sys_sigaction, sys_sigprocmask, sys_sigtimedwait},
    sys::{
        sys_getegid, sys_geteuid, sys_getgid, sys_getpgid, sys_getuid, sys_prlimit64, sys_uname,
    },
    task::{
        sys_chdir, sys_clone, sys_execve, sys_exit, sys_futex, sys_getcwd, sys_getpid, sys_getppid,
        sys_gettid, sys_sched_yield, sys_set_tid_address, sys_wait4,
    },
    time::{sys_gettime, sys_gettimeofday, sys_nanosleep, sys_times},
};

pub async fn syscall(call_type: usize, args: [usize; 7]) -> Result<usize, LinuxError> {
    match call_type {
        SYS_GETCWD => sys_getcwd(args[0] as _, args[1] as _).await,
        SYS_CHDIR => sys_chdir(args[0] as _).await,
        SYS_OPENAT => sys_openat(args[0] as _, args[1] as _, args[2] as _, args[3] as _).await,
        SYS_DUP => sys_dup(args[0]).await,
        SYS_DUP3 => sys_dup3(args[0], args[1]).await,
        SYS_CLOSE => sys_close(args[0] as _).await,
        SYS_MKDIRAT => sys_mkdir_at(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_READ => sys_read(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_WRITE => sys_write(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_EXECVE => sys_execve(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_EXIT => sys_exit(args[0] as _),
        SYS_BRK => sys_brk(args[0] as _).await,
        SYS_GETPID => sys_getpid().await,
        SYS_PIPE2 => sys_pipe2(args[0] as _, args[1] as _).await,
        SYS_GETTIMEOFDAY => sys_gettimeofday(args[0] as _, args[1] as _).await,
        SYS_NANOSLEEP => sys_nanosleep(args[0] as _, args[1] as _).await,
        SYS_UNAME => sys_uname(args[0] as _).await,
        SYS_UNLINKAT => sys_unlinkat(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_FSTAT => sys_fstat(args[0] as _, args[1] as _).await,
        SYS_CLONE => {
            sys_clone(
                args[0] as _,
                args[1] as _,
                args[2] as _,
                args[3] as _,
                args[4] as _,
            )
            .await
        }
        SYS_WAIT4 => sys_wait4(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_SCHED_YIELD => sys_sched_yield().await,
        SYS_GETPPID => sys_getppid().await,
        SYS_MOUNT => {
            sys_mount(
                args[0] as _,
                args[1] as _,
                args[2] as _,
                args[3] as _,
                args[4] as _,
            )
            .await
        }
        SYS_UMOUNT2 => sys_umount2(args[0] as _, args[1] as _).await,
        SYS_MMAP => {
            sys_mmap(
                args[0] as _,
                args[1] as _,
                args[2] as _,
                args[3] as _,
                args[4] as _,
                args[5] as _,
            )
            .await
        }
        SYS_MUNMAP => sys_munmap(args[0] as _, args[1] as _).await,
        SYS_TIMES => sys_times(args[0] as _).await,
        SYS_GETDENTS => sys_getdents64(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_SET_TID_ADDRESS => sys_set_tid_address(args[0] as _).await,
        SYS_GETTID => sys_gettid().await,
        SYS_LSEEK => sys_lseek(args[0] as _, args[1] as _, args[2] as _),
        SYS_GETTIME => sys_gettime(args[0] as _, args[1] as _).await,
        SYS_SIGTIMEDWAIT => sys_sigtimedwait().await,
        SYS_PRLIMIT64 => {
            sys_prlimit64(args[0] as _, args[1] as _, args[2] as _, args[3] as _).await
        }
        SYS_READV => sys_readv(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_WRITEV => sys_writev(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_STATFS => sys_statfs(args[0] as _, args[1] as _).await,
        SYS_PREAD => sys_pread(args[0] as _, args[1] as _, args[2] as _, args[3] as _).await,
        SYS_FSTATAT => sys_fstatat(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_GETEUID => sys_geteuid().await,
        SYS_GETEGID => sys_getegid().await,
        SYS_GETGID => sys_getgid().await,
        SYS_GETUID => sys_getuid().await,
        SYS_GETPGID => sys_getpgid().await,
        SYS_IOCTL => {
            sys_ioctl(
                args[0] as _,
                args[1] as _,
                args[2] as _,
                args[3] as _,
                args[4] as _,
            )
            .await
        }
        SYS_FCNTL => sys_fcntl(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_UTIMEAT => sys_utimensat(args[0] as _, args[1] as _, args[2] as _, args[3] as _).await,
        SYS_SIGPROCMASK => sys_sigprocmask(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_SIGACTION => sys_sigaction(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_MPROTECT => sys_mprotect(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_FUTEX => {
            sys_futex(
                args[0] as _,
                args[1] as _,
                args[2] as _,
                args[3] as _,
                args[4] as _,
            )
            .await
        }
        _ => {
            warn!("unsupported syscall: {}", call_type);
            Err(LinuxError::EPERM)
        }
    }
}
