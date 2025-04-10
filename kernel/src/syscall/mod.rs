mod fd;
mod mm;
mod shm;
mod signal;
mod socket;
mod sys;
mod task;
mod time;
pub mod types;

pub use socket::NET_SERVER;
use syscalls::{Errno, Sysno};
pub use task::{cache_task_template, exec_with_process};

use log::warn;

use crate::user::UserTaskContainer;

type SysResult = Result<usize, Errno>;

impl UserTaskContainer {
    pub async fn syscall(&self, call_id: usize, args: [usize; 6]) -> Result<usize, Errno> {
        let sysno = Sysno::new(call_id).ok_or(Errno::EINVAL)?;
        match sysno {
            Sysno::getcwd => self.sys_getcwd(args[0].into(), args[1] as _).await,
            Sysno::chdir => self.sys_chdir(args[0].into()).await,
            Sysno::openat => {
                self.sys_openat(args[0] as _, args[1].into(), args[2] as _, args[3] as _)
                    .await
            }
            Sysno::dup => self.sys_dup(args[0]).await,
            Sysno::dup3 => self.sys_dup3(args[0], args[1]).await,
            Sysno::close => self.sys_close(args[0] as _).await,
            Sysno::mkdirat => {
                self.sys_mkdir_at(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::read => {
                self.sys_read(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::write => {
                self.sys_write(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::execve => {
                self.sys_execve(args[0].into(), args[1].into(), args[2].into())
                    .await
            }
            Sysno::exit => self.sys_exit(args[0] as _).await,
            Sysno::brk => self.sys_brk(args[0] as _).await,
            Sysno::getpid => self.sys_getpid().await,
            Sysno::pipe2 => self.sys_pipe2(args[0].into(), args[1] as _).await,
            Sysno::gettimeofday => self.sys_gettimeofday(args[0].into(), args[1] as _).await,
            Sysno::nanosleep => self.sys_nanosleep(args[0].into(), args[1].into()).await,
            Sysno::uname => self.sys_uname(args[0].into()).await,
            Sysno::unlinkat => {
                self.sys_unlinkat(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::fstat => self.sys_fstat(args[0] as _, args[1].into()).await,
            Sysno::wait4 => {
                self.sys_wait4(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::sched_yield => self.sys_sched_yield().await,
            Sysno::getppid => self.sys_getppid().await,
            Sysno::mount => {
                self.sys_mount(
                    args[0].into(),
                    args[1].into(),
                    args[2].into(),
                    args[3] as _,
                    args[4] as _,
                )
                .await
            }
            Sysno::umount2 => self.sys_umount2(args[0].into(), args[1] as _).await,
            Sysno::mmap => {
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
            Sysno::munmap => self.sys_munmap(args[0] as _, args[1] as _).await,
            Sysno::times => self.sys_times(args[0].into()).await,
            Sysno::getdents64 => {
                self.sys_getdents64(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::set_tid_address => self.sys_set_tid_address(args[0] as _).await,
            Sysno::gettid => self.sys_gettid().await,
            Sysno::lseek => self.sys_lseek(args[0] as _, args[1] as _, args[2] as _),
            Sysno::clock_gettime => self.sys_clock_gettime(args[0] as _, args[1].into()).await,
            Sysno::rt_sigtimedwait => self.sys_sigtimedwait().await,
            Sysno::rt_sigsuspend => self.sys_sigsuspend(args[0].into()).await,
            Sysno::prlimit64 => {
                self.sys_prlimit64(args[0] as _, args[1] as _, args[2].into(), args[3].into())
                    .await
            }
            Sysno::readv => {
                self.sys_readv(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::writev => {
                self.sys_writev(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::statfs => self.sys_statfs(args[0].into(), args[1].into()).await,
            Sysno::pread64 => {
                self.sys_pread(args[0] as _, args[1].into(), args[2] as _, args[3] as _)
                    .await
            }
            Sysno::pwrite64 => {
                self.sys_pwrite(args[0] as _, args[1].into(), args[2] as _, args[3] as _)
                    .await
            }
            #[cfg(not(target_arch = "x86_64"))]
            Sysno::fstatat => {
                self.sys_fstatat(args[0] as _, args[1].into(), args[2].into())
                    .await
            }
            #[cfg(target_arch = "x86_64")]
            Sysno::newfstatat => {
                self.sys_fstatat(args[0] as _, args[1].into(), args[2].into())
                    .await
            }
            Sysno::geteuid => self.sys_geteuid().await,
            Sysno::getegid => self.sys_getegid().await,
            Sysno::getgid => self.sys_getgid().await,
            Sysno::getuid => self.sys_getuid().await,
            Sysno::getpgid => self.sys_getpgid().await,
            Sysno::ioctl => {
                self.sys_ioctl(
                    args[0] as _,
                    args[1] as _,
                    args[2] as _,
                    args[3] as _,
                    args[4] as _,
                )
                .await
            }
            Sysno::fcntl => {
                self.sys_fcntl(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            Sysno::utimensat => {
                self.sys_utimensat(args[0] as _, args[1].into(), args[2].into(), args[3] as _)
                    .await
            }
            Sysno::rt_sigprocmask => {
                self.sys_sigprocmask(args[0] as _, args[1].into(), args[2].into())
                    .await
            }
            Sysno::rt_sigaction => {
                self.sys_sigaction(args[0] as _, args[1].into(), args[2].into())
                    .await
            }
            Sysno::mprotect => {
                self.sys_mprotect(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            Sysno::futex => {
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
            Sysno::readlinkat => {
                self.sys_readlinkat(args[0] as _, args[1].into(), args[2].into(), args[3] as _)
                    .await
            }
            Sysno::sendfile => {
                self.sys_sendfile(args[0] as _, args[1] as _, args[2] as _, args[3] as _)
                    .await
            }
            Sysno::tkill => self.sys_tkill(args[0] as _, args[1] as _).await,
            Sysno::rt_sigreturn => self.sys_sigreturn().await,
            Sysno::get_robust_list => {
                warn!("SYS_GET_ROBUST_LIST @ ");
                Ok(0)
            } // always ok for now
            Sysno::ppoll => {
                self.sys_ppoll(args[0].into(), args[1] as _, args[2].into(), args[3] as _)
                    .await
            }
            Sysno::getrusage => self.sys_getrusage(args[0] as _, args[1].into()).await,
            Sysno::setpgid => self.sys_setpgid(args[0] as _, args[1] as _).await,
            Sysno::pselect6 => {
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
            Sysno::kill => self.sys_kill(args[0] as _, args[1] as _).await,
            Sysno::fsync => Ok(0),
            Sysno::faccessat => {
                self.sys_faccess_at(args[0] as _, args[1].into(), args[2], args[3])
                    .await
            } // always be ok at now.
            Sysno::faccessat2 => Ok(0),
            Sysno::socket => {
                self.sys_socket(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            Sysno::socketpair => {
                self.sys_socket_pair(args[0] as _, args[1] as _, args[2] as _, args[3].into())
                    .await
            }
            Sysno::bind => {
                self.sys_bind(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::listen => self.sys_listen(args[0] as _, args[1] as _).await,
            Sysno::accept => {
                self.sys_accept(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            Sysno::accept4 => {
                self.sys_accept4(args[0] as _, args[1].into(), args[2] as _, args[3] as _)
                    .await
            }
            Sysno::connect => {
                self.sys_connect(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::recvfrom => {
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
            Sysno::sendto => {
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
            Sysno::syslog => {
                self.sys_klogctl(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::sysinfo => self.sys_info(args[0].into()).await,
            Sysno::msync => self.sys_msync(args[0], args[1], args[2] as _).await,
            Sysno::exit_group => self.sys_exit_group(args[0]),
            Sysno::ftruncate => self.sys_ftruncate(args[0], args[1]).await,
            Sysno::shmget => {
                self.sys_shmget(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            Sysno::shmat => {
                self.sys_shmat(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            Sysno::shmctl => {
                self.sys_shmctl(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            Sysno::setitimer => {
                self.sys_setitimer(args[0] as _, args[1].into(), args[2].into())
                    .await
            }
            Sysno::setsockopt => {
                self.sys_setsockopt(
                    args[0] as _,
                    args[1] as _,
                    args[2] as _,
                    args[3] as _,
                    args[4] as _,
                )
                .await
            }
            Sysno::getsockopt => {
                self.sys_getsockopt(
                    args[0] as _,
                    args[1] as _,
                    args[2] as _,
                    args[3].into(),
                    args[4].into(),
                )
                .await
            }
            Sysno::getsockname => {
                self.sys_getsockname(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::getpeername => {
                self.sys_getpeername(args[0] as _, args[1].into(), args[2] as _)
                    .await
            }
            Sysno::setsid => self.sys_setsid().await,
            Sysno::shutdown => self.sys_shutdown(args[0] as _, args[1] as _).await,
            Sysno::sched_getparam => self.sys_sched_getparam(args[0] as _, args[1] as _).await,
            Sysno::sched_setscheduler => {
                self.sys_sched_setscheduler(args[0] as _, args[1] as _, args[2] as _)
                    .await
            }
            Sysno::clock_getres => self.sys_clock_getres(args[0] as _, args[1].into()).await,
            Sysno::clock_nanosleep => {
                self.sys_clock_nanosleep(args[0] as _, args[1] as _, args[2].into(), args[3].into())
                    .await
            }
            Sysno::epoll_create1 => self.sys_epoll_create1(args[0] as _).await,
            Sysno::epoll_ctl => {
                self.sys_epoll_ctl(args[0] as _, args[1] as _, args[2] as _, args[3].into())
                    .await
            }
            Sysno::epoll_pwait => {
                self.sys_epoll_wait(
                    args[0] as _,
                    args[1].into(),
                    args[2] as _,
                    args[3] as _,
                    args[4] as _,
                )
                .await
            }
            Sysno::copy_file_range => {
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
            Sysno::getrandom => {
                self.sys_getrandom(args[0].into(), args[1] as _, args[2] as _)
                    .await
            }
            Sysno::sched_setaffinity => {
                log::debug!("sys_setaffinity() ");
                Ok(0)
            }
            Sysno::sched_getscheduler => {
                log::debug!("sys_sched_getscheduler");
                Ok(0)
            }
            Sysno::sched_getaffinity => {
                self.sys_sched_getaffinity(args[0], args[1], args[2].into())
                    .await
            }
            Sysno::setgroups => Ok(0),
            Sysno::renameat2 => {
                self.sys_renameat2(args[0], args[1].into(), args[2], args[3].into(), args[4])
                    .await
            }
            #[cfg(not(any(target_arch = "x86_64")))]
            Sysno::clone => {
                self.sys_clone(
                    args[0] as _,
                    args[1] as _,
                    args[2].into(),
                    args[3] as _,
                    args[4].into(),
                )
                .await
            }
            #[cfg(any(target_arch = "x86_64"))]
            Sysno::clone => {
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
            Sysno::select => {
                self.sys_select(
                    args[0] as _,
                    args[1].into(),
                    args[2].into(),
                    args[3].into(),
                    args[4].into(),
                )
                .await
            }
            #[cfg(target_arch = "x86_64")]
            Sysno::mkdir => self.sys_mkdir(args[0].into(), args[1]).await,
            #[cfg(target_arch = "x86_64")]
            Sysno::readlink => {
                self.sys_readlink(args[0].into(), args[1].into(), args[2])
                    .await
            }
            #[cfg(target_arch = "x86_64")]
            Sysno::arch_prctl => self.sys_arch_prctl(args[0], args[1]).await,
            #[cfg(target_arch = "x86_64")]
            Sysno::open => self.sys_open(args[0].into(), args[1], args[2]).await,
            #[cfg(target_arch = "x86_64")]
            Sysno::fork => self.sys_fork().await,
            #[cfg(target_arch = "x86_64")]
            Sysno::pipe => self.sys_pipe2(args[0].into(), 0).await,
            #[cfg(target_arch = "x86_64")]
            Sysno::unlink => self.sys_unlink(args[0].into()).await,
            #[cfg(target_arch = "x86_64")]
            Sysno::poll => self.sys_poll(args[0].into(), args[1], args[2] as _).await,
            #[cfg(target_arch = "x86_64")]
            Sysno::stat => self.sys_stat(args[0].into(), args[1].into()).await,
            #[cfg(target_arch = "x86_64")]
            Sysno::lstat => self.sys_lstat(args[0].into(), args[1].into()).await,
            #[cfg(target_arch = "x86_64")]
            Sysno::dup2 => self.sys_dup2(args[0], args[1]).await,
            _ => {
                warn!("unsupported syscall: {}", call_id);
                Err(Errno::EPERM)
            }
        }
    }
}
