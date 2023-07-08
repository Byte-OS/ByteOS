pub mod consts;
mod fd;
mod func;
mod mm;
mod shm;
mod signal;
mod socket;
mod sys;
mod task;
mod time;

pub use socket::NET_SERVER;
pub use task::exec_with_process;

use log::warn;

use self::{
    consts::{
        LinuxError, SYS_ACCEPT, SYS_BIND, SYS_BRK, SYS_CHDIR, SYS_CLONE, SYS_CLOSE, SYS_CONNECT,
        SYS_DUP, SYS_DUP3, SYS_EXECVE, SYS_EXIT, SYS_EXIT_GROUP, SYS_FACCESSAT, SYS_FACCESSAT2,
        SYS_FCNTL, SYS_FSTAT, SYS_FSTATAT, SYS_FSYNC, SYS_FTRUNCATE, SYS_FUTEX, SYS_GETCWD,
        SYS_GETDENTS, SYS_GETEGID, SYS_GETEUID, SYS_GETGID, SYS_GETPGID, SYS_GETPID, SYS_GETPPID,
        SYS_GETRUSAGE, SYS_GETSOCKNAME, SYS_GETTID, SYS_GETTIME, SYS_GETTIMEOFDAY, SYS_GETUID,
        SYS_GET_ROBUST_LIST, SYS_IOCTL, SYS_KILL, SYS_KLOGCTL, SYS_LISTEN, SYS_LSEEK, SYS_MKDIRAT,
        SYS_MMAP, SYS_MOUNT, SYS_MPROTECT, SYS_MSYNC, SYS_MUNMAP, SYS_NANOSLEEP, SYS_OPENAT,
        SYS_PIPE2, SYS_PPOLL, SYS_PREAD, SYS_PRLIMIT64, SYS_PSELECT, SYS_PWRITE, SYS_READ,
        SYS_READLINKAT, SYS_READV, SYS_RECVFROM, SYS_SCHED_YIELD, SYS_SENDFILE, SYS_SENDTO,
        SYS_SETITIMER, SYS_SETPGID, SYS_SETSOCKOPT, SYS_SET_TID_ADDRESS, SYS_SHMAT, SYS_SHMCTL,
        SYS_SHMGET, SYS_SIGACTION, SYS_SIGPROCMASK, SYS_SIGRETURN, SYS_SIGSUSPEND,
        SYS_SIGTIMEDWAIT, SYS_SOCKET, SYS_STATFS, SYS_SYSINFO, SYS_TIMES, SYS_TKILL, SYS_UMOUNT2,
        SYS_UNAME, SYS_UNLINKAT, SYS_UTIMEAT, SYS_WAIT4, SYS_WRITE, SYS_WRITEV,
    },
    fd::{
        sys_close, sys_dup, sys_dup3, sys_fcntl, sys_fstat, sys_fstatat, sys_ftruncate,
        sys_getdents64, sys_ioctl, sys_lseek, sys_mkdir_at, sys_mount, sys_openat, sys_pipe2,
        sys_ppoll, sys_pread, sys_pselect, sys_pwrite, sys_read, sys_readlinkat, sys_readv,
        sys_sendfile, sys_statfs, sys_umount2, sys_unlinkat, sys_utimensat, sys_write, sys_writev,
    },
    mm::{sys_brk, sys_mmap, sys_mprotect, sys_msync, sys_munmap},
    shm::{sys_shmat, sys_shmctl, sys_shmget},
    signal::{sys_sigaction, sys_sigprocmask, sys_sigsuspend, sys_sigtimedwait},
    socket::{
        sys_accept, sys_bind, sys_connect, sys_getsockname, sys_listen, sys_recvfrom, sys_sendto,
        sys_setsockopt, sys_socket,
    },
    sys::{
        sys_getegid, sys_geteuid, sys_getgid, sys_getpgid, sys_getuid, sys_info, sys_klogctl,
        sys_prlimit64, sys_setpgid, sys_uname,
    },
    task::{
        sys_chdir, sys_clone, sys_execve, sys_exit, sys_exit_group, sys_futex, sys_getcwd,
        sys_getpid, sys_getppid, sys_getrusage, sys_gettid, sys_kill, sys_sched_yield,
        sys_set_tid_address, sys_sigreturn, sys_tkill, sys_wait4,
    },
    time::{sys_clock_gettime, sys_gettimeofday, sys_nanosleep, sys_setitimer, sys_times},
};

pub async fn syscall(call_type: usize, args: [usize; 7]) -> Result<usize, LinuxError> {
    match call_type {
        SYS_GETCWD => sys_getcwd(args[0].into(), args[1] as _).await,
        SYS_CHDIR => sys_chdir(args[0].into()).await,
        SYS_OPENAT => sys_openat(args[0] as _, args[1].into(), args[2] as _, args[3] as _).await,
        SYS_DUP => sys_dup(args[0]).await,
        SYS_DUP3 => sys_dup3(args[0], args[1]).await,
        SYS_CLOSE => sys_close(args[0] as _).await,
        SYS_MKDIRAT => sys_mkdir_at(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_READ => sys_read(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_WRITE => sys_write(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_EXECVE => sys_execve(args[0].into(), args[1].into(), args[2].into()).await,
        SYS_EXIT => sys_exit(args[0] as _),
        SYS_BRK => sys_brk(args[0] as _).await,
        SYS_GETPID => sys_getpid().await,
        SYS_PIPE2 => sys_pipe2(args[0].into(), args[1] as _).await,
        SYS_GETTIMEOFDAY => sys_gettimeofday(args[0].into(), args[1] as _).await,
        SYS_NANOSLEEP => sys_nanosleep(args[0].into(), args[1].into()).await,
        SYS_UNAME => sys_uname(args[0].into()).await,
        SYS_UNLINKAT => sys_unlinkat(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_FSTAT => sys_fstat(args[0] as _, args[1].into()).await,
        SYS_CLONE => {
            sys_clone(
                args[0] as _,
                args[1] as _,
                args[2].into(),
                args[3] as _,
                args[4].into(),
            )
            .await
        }
        SYS_WAIT4 => sys_wait4(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_SCHED_YIELD => sys_sched_yield().await,
        SYS_GETPPID => sys_getppid().await,
        SYS_MOUNT => {
            sys_mount(
                args[0].into(),
                args[1].into(),
                args[2].into(),
                args[3] as _,
                args[4] as _,
            )
            .await
        }
        SYS_UMOUNT2 => sys_umount2(args[0].into(), args[1] as _).await,
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
        SYS_TIMES => sys_times(args[0].into()).await,
        SYS_GETDENTS => sys_getdents64(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_SET_TID_ADDRESS => sys_set_tid_address(args[0] as _).await,
        SYS_GETTID => sys_gettid().await,
        SYS_LSEEK => sys_lseek(args[0] as _, args[1] as _, args[2] as _),
        SYS_GETTIME => sys_clock_gettime(args[0] as _, args[1].into()).await,
        SYS_SIGTIMEDWAIT => sys_sigtimedwait().await,
        SYS_SIGSUSPEND => sys_sigsuspend(args[0].into()).await,
        SYS_PRLIMIT64 => {
            sys_prlimit64(args[0] as _, args[1] as _, args[2].into(), args[3].into()).await
        }
        SYS_READV => sys_readv(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_WRITEV => sys_writev(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_STATFS => sys_statfs(args[0].into(), args[1].into()).await,
        SYS_PREAD => sys_pread(args[0] as _, args[1].into(), args[2] as _, args[3] as _).await,
        SYS_PWRITE => sys_pwrite(args[0] as _, args[1].into(), args[2] as _, args[3] as _).await,
        SYS_FSTATAT => sys_fstatat(args[0] as _, args[1].into(), args[2].into()).await,
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
        SYS_UTIMEAT => {
            sys_utimensat(args[0] as _, args[1].into(), args[2].into(), args[3] as _).await
        }
        SYS_SIGPROCMASK => sys_sigprocmask(args[0] as _, args[1].into(), args[2].into()).await,
        SYS_SIGACTION => sys_sigaction(args[0] as _, args[1].into(), args[2].into()).await,
        SYS_MPROTECT => sys_mprotect(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_FUTEX => {
            sys_futex(
                args[0].into(),
                args[1] as _,
                args[2] as _,
                args[3] as _,
                args[4] as _,
                args[5] as _,
            )
            .await
        }
        SYS_READLINKAT => {
            sys_readlinkat(args[0] as _, args[1].into(), args[2].into(), args[3] as _).await
        }
        SYS_SENDFILE => sys_sendfile(args[0] as _, args[1] as _, args[2] as _, args[3] as _).await,
        SYS_TKILL => sys_tkill(args[0] as _, args[1] as _).await,
        SYS_SIGRETURN => sys_sigreturn().await,
        SYS_GET_ROBUST_LIST => {
            warn!("SYS_GET_ROBUST_LIST @ ");
            Ok(0)
        } // always ok for now
        SYS_PPOLL => sys_ppoll(args[0].into(), args[1] as _, args[2].into(), args[3] as _).await,
        SYS_GETRUSAGE => sys_getrusage(args[0] as _, args[1].into()).await,
        SYS_SETPGID => sys_setpgid(args[0] as _, args[1] as _).await,
        SYS_PSELECT => {
            sys_pselect(
                args[0] as _,
                args[1].into(),
                args[2].into(),
                args[3].into(),
                args[4].into(),
                args[5] as _,
            )
            .await
        }
        SYS_KILL => sys_kill(args[0] as _, args[1] as _).await,
        SYS_FSYNC => Ok(0),
        SYS_FACCESSAT => Ok(0), // always be ok at now.
        SYS_FACCESSAT2 => Ok(0),
        SYS_SOCKET => sys_socket(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_BIND => sys_bind(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_LISTEN => sys_listen(args[0] as _, args[1] as _).await,
        SYS_ACCEPT => sys_accept(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_CONNECT => sys_connect(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_RECVFROM => {
            sys_recvfrom(
                args[0] as _,
                args[1].into(),
                args[2] as _,
                args[3] as _,
                args[4].into(),
                args[5].into(),
            )
            .await
        }
        SYS_SENDTO => {
            sys_sendto(
                args[0] as _,
                args[1].into(),
                args[2] as _,
                args[3] as _,
                args[4].into(),
                args[5].into(),
            )
            .await
        }
        SYS_KLOGCTL => sys_klogctl(args[0] as _, args[1].into(), args[2] as _).await,
        SYS_SYSINFO => sys_info(args[0].into()).await,
        SYS_MSYNC => sys_msync(args[0], args[1], args[2] as _).await,
        SYS_EXIT_GROUP => sys_exit_group(args[0]),
        SYS_FTRUNCATE => sys_ftruncate(args[0], args[1]).await,
        SYS_SHMGET => sys_shmget(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_SHMAT => sys_shmat(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_SHMCTL => sys_shmctl(args[0] as _, args[1] as _, args[2] as _).await,
        SYS_SETITIMER => sys_setitimer(args[0] as _, args[1].into(), args[2].into()).await,
        SYS_SETSOCKOPT => {
            sys_setsockopt(
                args[0] as _,
                args[1] as _,
                args[2] as _,
                args[3] as _,
                args[4] as _,
            )
            .await
        }
        SYS_GETSOCKNAME => sys_getsockname(args[0] as _, args[1].into(), args[2] as _).await,
        _ => {
            warn!("unsupported syscall: {}", call_type);
            Err(LinuxError::EPERM)
        }
    }
}
