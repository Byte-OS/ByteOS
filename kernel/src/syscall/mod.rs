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

use crate::user::UserTaskContainer;

use self::consts::*;

type SysResult = Result<usize, LinuxError>;

impl UserTaskContainer {
    pub async fn syscall(&self, call_id: usize, args: [usize; 6]) -> Result<usize, LinuxError> {
        match call_id {
            SYS_GETCWD => self.sys_getcwd(args[0].into(), args[1] as _).await,
            SYS_CHDIR => self.sys_chdir(args[0].into()).await,
            SYS_OPENAT => {
                self.sys_openat(args[0] as _, args[1].into(), args[2] as _, args[3] as _)
                    .await
            }
            SYS_DUP => self.sys_dup(args[0]).await,
            SYS_DUP3 => self.sys_dup3(args[0], args[1]).await,
            SYS_CLOSE => self.sys_close(args[0] as _).await,
            SYS_MKDIRAT => {
                self.sys_mkdir_at(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_READ => {
                self.sys_read(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_WRITE => {
                self.sys_write(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_EXECVE => {
                self.sys_execve(args[0].into(), args[1].into(), args[2].into())
                    .await
            }
            SYS_EXIT => self.sys_exit(args[0] as _).await,
            SYS_BRK => self.sys_brk(args[0] as _).await,
            SYS_GETPID => self.sys_getpid().await,
            SYS_PIPE2 => self.sys_pipe2(args[0].into(), args[1] as _).await,
            SYS_GETTIMEOFDAY => self.sys_gettimeofday(args[0].into(), args[1] as _).await,
            SYS_NANOSLEEP => self.sys_nanosleep(args[0].into(), args[1].into()).await,
            SYS_UNAME => self.sys_uname(args[0].into()).await,
            SYS_UNLINKAT => {
                self.sys_unlinkat(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_FSTAT => self.sys_fstat(args[0] as _, args[1].into()).await,
            SYS_WAIT4 => {
                self.sys_wait4(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_SCHED_YIELD => self.sys_sched_yield().await,
            SYS_GETPPID => self.sys_getppid().await,
            SYS_MOUNT => {
                self.sys_mount(
                    args[0].into(),
                    args[1].into(),
                    args[2].into(),
                    args[3] as _,
                    args[4] as _,
                )
                .await
            }
            SYS_UMOUNT2 => self.sys_umount2(args[0].into(), args[1] as _).await,
            SYS_MMAP => {
                self.sys_mmap(
                    args[0] as _,
                    args[1] as _,
                    args[2] as _,
                    args[3] as _,
                    args[4] as _,
                    args[5] as _,
                )
                .await
            }
            SYS_MUNMAP => self.sys_munmap(args[0] as _, args[1] as _).await,
            SYS_TIMES => self.sys_times(args[0].into()).await,
            SYS_GETDENTS => {
                self.sys_getdents64(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_SET_TID_ADDRESS => self.sys_set_tid_address(args[0] as _).await,
            SYS_GETTID => self.sys_gettid().await,
            SYS_LSEEK => self.sys_lseek(args[0] as _, args[1] as _, args[2] as _),
            SYS_GETTIME => self.sys_clock_gettime(args[0] as _, args[1].into()).await,
            SYS_SIGTIMEDWAIT => self.sys_sigtimedwait().await,
            SYS_SIGSUSPEND => self.sys_sigsuspend(args[0].into()).await,
            SYS_PRLIMIT64 => {
                self.sys_prlimit64(args[0] as _, args[1] as _, args[2].into(), args[3].into())
                    .await
            }
            SYS_READV => {
                self.sys_readv(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_WRITEV => {
                self.sys_writev(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_STATFS => self.sys_statfs(args[0].into(), args[1].into()).await,
            SYS_PREAD => {
                self.sys_pread(args[0] as _, args[1].into(), args[2] as _, args[3] as _)
                    .await
            }
            SYS_PWRITE => {
                self.sys_pwrite(args[0] as _, args[1].into(), args[2] as _, args[3] as _)
                    .await
            }
            SYS_FSTATAT => {
                self.sys_fstatat(args[0] as _, args[1].into(), args[2].into())
                    .await
            }
            SYS_GETEUID => self.sys_geteuid().await,
            SYS_GETEGID => self.sys_getegid().await,
            SYS_GETGID => self.sys_getgid().await,
            SYS_GETUID => self.sys_getuid().await,
            SYS_GETPGID => self.sys_getpgid().await,
            SYS_IOCTL => {
                self.sys_ioctl(
                    args[0] as _,
                    args[1] as _,
                    args[2] as _,
                    args[3] as _,
                    args[4] as _,
                )
                .await
            }
            SYS_FCNTL => {
                self.sys_fcntl(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            SYS_UTIMEAT => {
                self.sys_utimensat(args[0] as _, args[1].into(), args[2].into(), args[3] as _)
                    .await
            }
            SYS_SIGPROCMASK => {
                self.sys_sigprocmask(args[0] as _, args[1].into(), args[2].into())
                    .await
            }
            SYS_SIGACTION => {
                self.sys_sigaction(args[0] as _, args[1].into(), args[2].into())
                    .await
            }
            SYS_MPROTECT => {
                self.sys_mprotect(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            SYS_FUTEX => {
                self.sys_futex(
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
                self.sys_readlinkat(args[0] as _, args[1].into(), args[2].into(), args[3] as _)
                    .await
            }
            SYS_SENDFILE => {
                self.sys_sendfile(args[0] as _, args[1] as _, args[2] as _, args[3] as _)
                    .await
            }
            SYS_TKILL => self.sys_tkill(args[0] as _, args[1] as _).await,
            SYS_SIGRETURN => self.sys_sigreturn().await,
            SYS_GET_ROBUST_LIST => {
                warn!("SYS_GET_ROBUST_LIST @ ");
                Ok(0)
            } // always ok for now
            SYS_PPOLL => {
                self.sys_ppoll(args[0].into(), args[1] as _, args[2].into(), args[3] as _)
                    .await
            }
            SYS_GETRUSAGE => self.sys_getrusage(args[0] as _, args[1].into()).await,
            SYS_SETPGID => self.sys_setpgid(args[0] as _, args[1] as _).await,
            SYS_PSELECT => {
                self.sys_pselect(
                    args[0] as _,
                    args[1].into(),
                    args[2].into(),
                    args[3].into(),
                    args[4].into(),
                    args[5] as _,
                )
                .await
            }
            SYS_KILL => self.sys_kill(args[0] as _, args[1] as _).await,
            SYS_FSYNC => Ok(0),
            SYS_FACCESSAT => {
                self.sys_faccess_at(args[0] as _, args[1].into(), args[2], args[3])
                    .await
            } // always be ok at now.
            SYS_FACCESSAT2 => Ok(0),
            SYS_SOCKET => {
                self.sys_socket(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            SYS_SOCKETPAIR => {
                self.sys_socket_pair(args[0] as _, args[1] as _, args[2] as _, args[3] as _)
                    .await
            }
            SYS_BIND => {
                self.sys_bind(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_LISTEN => self.sys_listen(args[0] as _, args[1] as _).await,
            SYS_ACCEPT => {
                self.sys_accept(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            SYS_ACCEPT4 => {
                self.sys_accept4(args[0] as _, args[1].into(), args[2] as _, args[3] as _)
                    .await
            }
            SYS_CONNECT => {
                self.sys_connect(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_RECVFROM => {
                self.sys_recvfrom(
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
                self.sys_sendto(
                    args[0] as _,
                    args[1].into(),
                    args[2] as _,
                    args[3] as _,
                    args[4].into(),
                    args[5].into(),
                )
                .await
            }
            SYS_KLOGCTL => {
                self.sys_klogctl(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_SYSINFO => self.sys_info(args[0].into()).await,
            SYS_MSYNC => self.sys_msync(args[0], args[1], args[2] as _).await,
            SYS_EXIT_GROUP => self.sys_exit_group(args[0]),
            SYS_FTRUNCATE => self.sys_ftruncate(args[0], args[1]).await,
            SYS_SHMGET => {
                self.sys_shmget(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            SYS_SHMAT => {
                self.sys_shmat(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            SYS_SHMCTL => {
                self.sys_shmctl(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            SYS_SETITIMER => {
                self.sys_setitimer(args[0] as _, args[1].into(), args[2].into())
                    .await
            }
            SYS_SETSOCKOPT => {
                self.sys_setsockopt(
                    args[0] as _,
                    args[1] as _,
                    args[2] as _,
                    args[3] as _,
                    args[4] as _,
                )
                .await
            }
            SYS_GETSOCKOPT => {
                self.sys_getsockopt(
                    args[0] as _,
                    args[1] as _,
                    args[2] as _,
                    args[3] as _,
                    args[4] as _,
                )
                .await
            }
            SYS_GETSOCKNAME => {
                self.sys_getsockname(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_GETPEERNAME => {
                self.sys_getpeername(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            SYS_SETSID => self.sys_setsid().await,
            SYS_SHUTDOWN => self.sys_shutdown(args[0] as _, args[1] as _).await,
            SYS_SCHED_GETPARAM => self.sys_sched_getparam(args[0] as _, args[1] as _).await,
            SYS_SCHED_SETSCHEDULER => {
                self.sys_sched_setscheduler(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            SYS_CLOCK_GETRES => self.sys_clock_getres(args[0] as _, args[1].into()).await,
            SYS_CLOCK_NANOSLEEP => {
                self.sys_clock_nanosleep(args[0] as _, args[1] as _, args[2].into(), args[3].into())
                    .await
            }
            SYS_EPOLL_CREATE => self.sys_epoll_create1(args[0] as _).await,
            SYS_EPOLL_CTL => {
                self.sys_epoll_ctl(args[0] as _, args[1] as _, args[2] as _, args[3].into())
                    .await
            }
            SYS_EPOLL_WAIT => {
                self.sys_epoll_wait(
                    args[0] as _,
                    args[1].into(),
                    args[2] as _,
                    args[3] as _,
                    args[4] as _,
                )
                .await
            }
            SYS_COPY_FILE_RANGE => {
                self.sys_copy_file_range(
                    args[0] as _,
                    args[1].into(),
                    args[2] as _,
                    args[3].into(),
                    args[4],
                    args[5] as _,
                )
                .await
            }
            SYS_GETRANDOM => {
                self.sys_getrandom(args[0].into(), args[1] as _, args[2] as _)
                    .await
            }
            SYS_SCHED_SETAFFINITY => {
                log::debug!("sys_getaffinity() ");
                Ok(0)
            }
            SYS_SCHED_GETSCHEDULER => {
                log::debug!("sys_sched_getscheduler");
                Ok(0)
            }
            #[cfg(not(target_arch = "x86_64"))]
            SYS_CLONE => {
                self.sys_clone(
                    args[0] as _,
                    args[1] as _,
                    args[2].into(),
                    args[3] as _,
                    args[4].into(),
                )
                .await
            }
            #[cfg(target_arch = "x86_64")]
            SYS_CLONE => {
                self.sys_clone(
                    args[0] as _,
                    args[1] as _,
                    args[2].into(),
                    args[4] as _,
                    args[3].into(),
                )
                .await
            }
            #[cfg(target_arch = "x86_64")]
            SYS_ARCH_PRCTL => self.sys_arch_prctl(args[0], args[1]).await,
            #[cfg(target_arch = "x86_64")]
            SYS_OPEN => self.sys_open(args[0].into(), args[1], args[2]).await,
            #[cfg(target_arch = "x86_64")]
            SYS_FORK => self.sys_fork().await,
            #[cfg(target_arch = "x86_64")]
            SYS_PIPE => self.sys_pipe2(args[0].into(), 0).await,
            #[cfg(target_arch = "x86_64")]
            SYS_UNLINK => self.sys_unlink(args[0].into()).await,
            #[cfg(target_arch = "x86_64")]
            SYS_POLL => self.sys_poll(args[0].into(), args[1], args[2] as _).await,
            #[cfg(target_arch = "x86_64")]
            SYS_STAT => self.sys_stat(args[0].into(), args[1].into()).await,
            #[cfg(target_arch = "x86_64")]
            SYS_LSTAT => self.sys_lstat(args[0].into(), args[1].into()).await,
            #[cfg(target_arch = "x86_64")]
            SYS_DUP2 => self.sys_dup2(args[0], args[1]).await,
            _ => {
                warn!("unsupported syscall: {}", call_id);
                Err(LinuxError::EPERM)
            }
        }
    }
}
