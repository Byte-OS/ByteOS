use executor::{current_task, current_user_task};
use log::debug;
use signal::{SigAction, SigMaskHow, SigProcMask};

use crate::tasks::WaitSignal;

use super::consts::{LinuxError, UserRef};

/*
 * 忽略信号：不采取任何操作、有两个信号不能被忽略：SIGKILL和SIGSTOP。
 * 注：SIGKILL和SIGSTOP这两个信号是不会被捕捉、阻塞、忽略的。
 * 捕获并处理信号：内核中断正在执行的代码，转去执行信号的处理函数。
 * 执行默认操作：默认操作通常是终止进程，这取决于被发送的信号。
 *
 *
 *  //struct sigaction 类型用来描述对信号的处理，定义如下：
 *
 *  struct sigaction
 *  {
 *      void     (*sa_handler)(int);
 *      void     (*sa_sigaction)(int, siginfo_t *, void *);
 *      sigset_t  sa_mask;
 *      int       sa_flags;
 *      void     (*sa_restorer)(void);
 *  };
 *  在这个结构体中，
 *  成员 sa_handler 是一个函数指针，包含一个信号处理函数的地址，与signal函数类似
 *  成员sa_sigaction 则是另一个信号处理函数，它有三个参数，可以获得关于信号的更详细的信息。
 *  int iSignNum : 传入的信号
 *  //siginfo_t *pSignInfo : 该信号相关的一些信息,他是一个结构体原型如下
 *  siginfo_t {
 *      int      si_signo;    /* 信号值，对所有信号有意义 */
 *      int      si_errno;    /* errno 值，对所有信号有意义 */
 *      int      si_code;     /* 信号产生的原因，对所有信号有意义 */
 *      int      si_trapno;   /* Trap number that caused
 *                               hardware-generated signal
 *                               (unused on most architectures) */
 *      pid_t    si_pid;      /* 发送信号的进程ID */
 *      uid_t    si_uid;      /* 发送信号进程的真实用户ID */
 *      int      si_status;   /* 对出状态，对SIGCHLD 有意义 */
 *      clock_t  si_utime;    /* 用户消耗的时间，对SIGCHLD有意义 */
 *      clock_t  si_stime;    /* 内核消耗的时间，对SIGCHLD有意义 */
 *      sigval_t si_value;    /* 信号值，对所有实时有意义，是一个联合数据结构，可以为一个整数（由si_int标示，也可以为一个指针，由si_ptr标示） */
 *      int      si_int;      /* POSIX.1b signal */
 *      void    *si_ptr;      /* POSIX.1b signal */
 *      int      si_overrun;  /* Timer overrun count; POSIX.1b timers */
 *      int      si_timerid;  /* Timer ID; POSIX.1b timers */
 *      void    *si_addr;     /* 触发fault的内存地址，对SIGILL,SIGFPE,SIGSEGV,SIGBUS 信号有意义 */
 *      long     si_band;     /* 对SIGPOLL信号有意义 */
 *      int      si_fd;       /* 对SIGPOLL信号有意义 */
 *      short    si_addr_lsb; /* Least significant bit of address
 *                               (since kernel 2.6.32) */
 *  }
 */

/// TODO: finish sigtimedwait
pub async fn sys_sigtimedwait() -> Result<usize, LinuxError> {
    debug!("sys_sigtimedwait @ ");
    WaitSignal(current_user_task()).await;
    // let task = current_user_task();
    // task.inner_map(|x| x.signal.has_signal());
    // Err(LinuxError::EAGAIN)
    Ok(0)
}

pub async fn sys_sigprocmask(
    how: usize,
    set: UserRef<SigProcMask>,
    oldset: UserRef<SigProcMask>,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_sigprocmask @ how: {:#x}, set: {}, oldset: {}",
        how, set, oldset
    );
    let user_task = current_task().as_user_task().unwrap();
    let how = SigMaskHow::from_usize(how).ok_or(LinuxError::EINVAL)?;

    let mut tcb = user_task.tcb.write();
    if oldset.is_valid() {
        let sigmask = oldset.get_mut();
        *sigmask = tcb.sigmask;
    }
    if set.is_valid() {
        let sigmask = set.get_mut();
        tcb.sigmask.handle(how, sigmask)
    }
    drop(tcb);
    // Err(LinuxError::EPERM)
    Ok(0)
}

/// 其次，每个线程都有自己独立的signal mask，但所有线程共享进程的signal action。这意味着，
/// 你可以在线程中调用pthread_sigmask(不是sigmask)来决定本线程阻塞哪些信号。
/// 但你不能调用sigaction来指定单个线程的信号处理方式。如果在某个线程中调用了sigaction处理某个信号，
/// 那么这个进程中的未阻塞这个信号的线程在收到这个信号都会按同一种方式处理这个信号。
/// 另外，注意子线程的mask是会从主线程继承而来的。

pub async fn sys_sigaction(
    sig: usize,
    act: UserRef<SigAction>,
    oldact: UserRef<SigAction>,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_sigaction @ sig: {}, act: {}, oldact: {}",
        sig, act, oldact
    );
    let user_task = current_task().as_user_task().unwrap();
    if oldact.is_valid() {
        *oldact.get_mut() = user_task.pcb.lock().sigaction[sig];
    }
    if act.is_valid() {
        user_task.pcb.lock().sigaction[sig] = *act.get_mut();
    }
    Ok(0)
}
