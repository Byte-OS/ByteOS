use super::{types::sys::Rusage, SysResult};
use crate::{
    syscall::{
        time::WaitUntilsec,
        types::{fd::FutexFlags, task::CloneFlags, time::TimeVal},
    },
    tasks::{
        exec::exec_with_process, futex_requeue, futex_wake, FileItem, UserTask, WaitFutex, WaitPid,
    },
    user::{entry::user_entry, UserTaskContainer},
    utils::{time::current_nsec, useref::UserRef},
};
use alloc::{
    string::{String, ToString},
    sync::Arc,
    sync::Weak,
    vec::Vec,
};
use core::cmp;
use executor::{select, thread, tid2task, yield_now, AsyncTask};
use fs::TimeSpec;
use log::{debug, warn};
use num_traits::FromPrimitive;
use polyhal::Time;
use polyhal_trap::trapframe::TrapFrameArgs;
use signal::SignalFlags;
use syscalls::Errno;
use vfscore::OpenFlags;

impl UserTaskContainer {
    pub async fn sys_chdir(&self, path_ptr: UserRef<i8>) -> SysResult {
        let path = path_ptr.get_cstr().map_err(|_| Errno::EINVAL)?;
        debug!("sys_chdir @ path: {}", path);
        let now_file = self.task.pcb.lock().curr_dir.clone();
        let new_dir = now_file.dentry_open(path, OpenFlags::O_DIRECTORY)?;

        match new_dir.metadata().unwrap().file_type {
            fs::FileType::Directory => {
                self.task.pcb.lock().curr_dir = new_dir;
                Ok(0)
            }
            _ => Err(Errno::ENOTDIR),
        }
    }

    pub async fn sys_getcwd(&self, buf_ptr: UserRef<u8>, size: usize) -> SysResult {
        debug!("sys_getcwd @ buffer_ptr{} size: {}", buf_ptr, size);
        let buffer = buf_ptr.slice_mut_with_len(size);
        let curr_path = self.task.pcb.lock().curr_dir.clone();
        let path = curr_path.path()?;
        let bytes = path.as_bytes();
        let len = cmp::min(bytes.len(), size);
        buffer[..len].copy_from_slice(&bytes[..len]);
        buffer[len..].fill(0);
        Ok(buf_ptr.into())
    }

    pub async fn sys_exit(&self, exit_code: isize) -> SysResult {
        debug!("sys_exit @ exit_code: {}  task_id: {}", exit_code, self.tid);
        // current_task().as_user_task().unwrap().exit(exit_code as _);
        self.task.thread_exit(exit_code as _);
        Ok(0)
    }

    pub async fn sys_execve(
        &self,
        filename: UserRef<i8>,      // *mut i8
        args: UserRef<UserRef<i8>>, // *mut *mut i8
        envp: UserRef<UserRef<i8>>, // *mut *mut i8
    ) -> SysResult {
        debug!(
            "sys_execve @ filename: {} args: {:?}: envp: {:?}",
            filename, args, envp
        );
        // TODO: use map_err insteads of unwrap and unsafe code.
        let filename = filename.get_cstr().map_err(|_| Errno::EINVAL)?;
        let args = args
            .slice_until_valid(|x| x.is_valid())
            .into_iter()
            .map(|x| x.get_cstr().unwrap().to_string())
            .collect();
        debug!("test1: envp: {:?}", envp);
        let envp: Vec<String> = envp
            .slice_until_valid(|x| x.is_valid())
            .into_iter()
            .map(|x| x.get_cstr().unwrap().to_string())
            .collect();
        debug!(
            "sys_execve @ filename: {} args: {:?}: envp: {:?}",
            filename, args, envp
        );

        // clear memory map
        // TODO: solve memory conflict
        // task.pcb.lock().memset.retain(|x| x.mtype == MemType::PTE);

        // check exec file.
        if filename == "/bin/true" {
            self.task.exit(0);
            return Ok(0);
        }
        let _exec_file = FileItem::fs_open(filename, OpenFlags::O_RDONLY)?;
        exec_with_process(self.task.clone(), filename.to_string(), args, envp).await?;
        self.task.before_run();
        Ok(0)
    }

    pub async fn sys_clone(
        &self,
        flags: usize,       // 复制 标志位
        stack: usize,       // 指定新的栈，可以为 0, 0 不处理
        ptid: UserRef<u32>, // 父线程 id
        tls: usize,         // TLS线程本地存储描述符
        ctid: UserRef<u32>, // 子线程 id
    ) -> SysResult {
        let sig = flags & 0xff;
        debug!(
            "[task {}] sys_clone @ flags: {:#x}, stack: {:#x}, ptid: {}, tls: {:#x}, ctid: {}",
            self.tid, flags, stack, ptid, tls, ctid
        );
        let flags = CloneFlags::from_bits_truncate(flags);
        debug!(
            "[task {}] sys_clone @ flags: {:?}, stack: {:#x}, ptid: {}, tls: {:#x}, ctid: {}",
            self.tid, flags, stack, ptid, tls, ctid
        );

        let new_task = match flags.contains(CloneFlags::CLONE_THREAD) {
            true => self.task.clone().thread_clone(),
            // false => curr_task.clone().fork(user_entry()),
            // use cow(Copy On Write) to save memory.
            false => self.task.clone().cow_fork(),
        };

        let clear_child_tid = flags
            .contains(CloneFlags::CLONE_CHILD_CLEARTID)
            .then_some(ctid)
            .unwrap_or(UserRef::from(0));

        let mut new_tcb = new_task.tcb.write();
        new_tcb.clear_child_tid = clear_child_tid.addr();

        if stack != 0 {
            new_tcb.cx[TrapFrameArgs::SP] = stack;
        }
        // set tls.
        if flags.contains(CloneFlags::CLONE_SETTLS) {
            new_tcb.cx[TrapFrameArgs::TLS] = tls;
        }
        if flags.contains(CloneFlags::CLONE_PARENT_SETTID) {
            *ptid.get_mut() = new_task.task_id as _;
        }
        if flags.contains(CloneFlags::CLONE_CHILD_SETTID) && ctid.is_valid() {
            *ctid.get_mut() = new_task.task_id as _;
        }
        new_tcb.exit_signal = sig as u8;
        drop(new_tcb);
        yield_now().await;
        thread::spawn(new_task.clone(), user_entry());
        Ok(new_task.task_id)
    }

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_fork(&self) -> SysResult {
        warn!("transfer syscall_fork to syscall_clone");
        self.sys_clone(0x11, 0, 0.into(), 0, 0.into()).await
    }

    pub async fn sys_wait4(
        &self,
        pid: isize,           // 指定进程ID，可为-1等待任何子进程；
        status: UserRef<i32>, // 接收状态的指针；
        options: usize,       // WNOHANG，WUNTRACED，WCONTINUED；
    ) -> SysResult {
        debug!(
            "[task {}] sys_wait4 @ pid: {}, status: {}, options: {}",
            self.tid, pid, status, options
        );

        // return LinuxError::ECHILD if there has no child process.
        if self.task.inner_map(|inner| inner.children.len()) == 0 {
            return Err(Errno::ECHILD);
        }

        if pid != -1 {
            self.task
                .inner_map(|inner| {
                    inner
                        .children
                        .iter()
                        .find(|x| x.task_id == pid as usize)
                        .map(|x| x.clone())
                })
                .ok_or(Errno::ECHILD)?;
        }
        if options == 0 || options == 2 || options == 3 || options == 10 {
            debug!(
                "children:{:?}",
                self.task.pcb.lock().children.iter().count()
            );
            let child_task = WaitPid(self.task.clone(), pid).await?;

            debug!(
                "wait ok: {}  waiter: {}",
                child_task.task_id, self.task.task_id
            );
            // release the task resources
            self.task
                .pcb
                .lock()
                .children
                .retain(|x| x.task_id != child_task.task_id);
            child_task.release();
            debug!("wait pid: {}", child_task.exit_code().unwrap());

            if status.is_valid() {
                *status.get_mut() = (child_task.exit_code().unwrap() as i32) << 8;
            }
            Ok(child_task.task_id)
        } else if options == 1 {
            let child_task = self
                .task
                .pcb
                .lock()
                .children
                .iter()
                .find(|x| x.task_id == pid as usize || pid == -1)
                .cloned();
            let exit = child_task.clone().map_or(None, |x| x.exit_code());
            match exit {
                Some(t1) => {
                    let child_task = child_task.unwrap();
                    // Release task.
                    self.task
                        .pcb
                        .lock()
                        .children
                        .retain(|x| x.task_id != child_task.task_id);
                    child_task.release();
                    if status.is_valid() {
                        *status.get_mut() = (t1 as i32) << 8;
                    }
                    // TIPS: This is a small change.
                    Ok(child_task.task_id)
                    // Ok(0)
                }
                None => Ok(0),
            }
        } else {
            warn!("wait4 unsupported options: {}", options);
            Err(Errno::EPERM)
        }
    }

    pub async fn sys_sched_yield(&self) -> SysResult {
        debug!("sys_sched_yield @ ");
        yield_now().await;
        Ok(0)
    }

    /// 对于每个线程，内核维护着两个属性(地址)，分别称为set_child_tid和clear_child_tid。默认情况下，这两个属性包含值NULL。

    /// set_child_tid
    /// 如果使用带有CLONE_CHILD_SETTID标志的clone(2)启动线程，则set_child_tid设置为该系统调用的ctid参数中传递的值。
    /// 设置set_child_tid时，新线程要做的第一件事就是在该地址写入其线程ID。
    /// clear_child_tid
    /// 如果使用带有CLONE_CHILD_CLEARTID标志的clone(2)启动线程，则clear_child_tid设置为该系统调用的ctid参数中传递的值。
    /// 系统调用set_tid_address()将调用线程的clear_child_tid值设置为tidptr。

    // 当clear_child_tid不为NULL的线程终止时，如果该线程与其他线程共享内存，则将0写入clear_child_tid中指定的地址，并且内核执行以下操作：

    // futex(clear_child_tid，FUTEX_WAKE，1 , NULL，NULL，0);

    // 此操作的效果是唤醒正在执行内存位置上的futex等待的单个线程。来自futex唤醒操作的错误将被忽略。
    pub async fn sys_set_tid_address(&self, tid_ptr: usize) -> SysResult {
        // information source: https://www.onitroad.com/jc/linux/man-pages/linux/man2/set_tid_address.2.html

        debug!("sys_set_tid_address @ tid_ptr: {:#x}", tid_ptr);
        self.task.tcb.write().clear_child_tid = tid_ptr;
        Ok(self.tid)
    }

    /// sys_getpid() 获取进程 id
    pub async fn sys_getpid(&self) -> SysResult {
        Ok(self.task.process_id)
    }

    /// sys_getppid() 获取父进程 id
    pub async fn sys_getppid(&self) -> SysResult {
        debug!("sys_getppid @ ");
        self.task
            .parent
            .read()
            .upgrade()
            .map(|x| x.task_id)
            .ok_or(Errno::EPERM)
    }

    /// sys_gettid() 获取线程 id.
    /// need to write correct clone and thread_clone for pthread.
    pub async fn sys_gettid(&self) -> SysResult {
        debug!("sys_gettid @ ");
        Ok(self.tid)
    }

    pub async fn sys_futex(
        &self,
        uaddr_ptr: UserRef<i32>,
        op: usize,
        value: usize,
        value2: usize,
        uaddr2: usize,
        value3: usize,
    ) -> SysResult {
        let op = if op >= 0x80 { op - 0x80 } else { op };
        debug!(
            "[task {}] sys_futex @ uaddr: {} op: {} value: {:#x}, value2: {:#x}, uaddr2: {:#x} , value3: {:#x}",
            self.tid, uaddr_ptr, op, value, value2, uaddr2, value3
        );
        let uaddr = uaddr_ptr.get_mut();
        let flags = FromPrimitive::from_usize(op).ok_or(Errno::EINVAL)?;
        debug!(
            "sys_futex @ uaddr: {:#x} flags: {:?} value: {}",
            uaddr, flags, value
        );

        match flags {
            FutexFlags::Wait => {
                if *uaddr == value as _ {
                    let futex_table = self.task.pcb.lock().futex_table.clone();
                    let mut table = futex_table.lock();
                    match table.get_mut(&uaddr_ptr.addr()) {
                        Some(t) => t.push(self.tid),
                        None => {
                            table.insert(uaddr_ptr.addr(), vec![self.tid]);
                        }
                    }
                    drop(table);
                    let wait_func = WaitFutex(futex_table.clone(), self.tid);
                    if value2 != 0 {
                        let timeout = UserRef::<TimeSpec>::from(value2).get_mut();
                        match select(wait_func, WaitUntilsec(current_nsec() + timeout.to_nsec()))
                            .await
                        {
                            executor::Either::Left((res, _)) => res,
                            executor::Either::Right(_) => Err(Errno::ETIMEDOUT),
                        }
                    } else {
                        wait_func.await
                    }
                    // wait_func.await
                } else {
                    Err(Errno::EAGAIN)
                }
            }
            FutexFlags::Wake => {
                let futex_table = self.task.pcb.lock().futex_table.clone();
                let count = futex_wake(futex_table, uaddr_ptr.addr(), value);
                yield_now().await;
                Ok(count)
            }
            FutexFlags::Requeue => {
                let futex_table = self.task.pcb.lock().futex_table.clone();
                Ok(futex_requeue(
                    futex_table,
                    uaddr_ptr.addr(),
                    value,
                    uaddr2,
                    value2,
                ))
            }
            _ => {
                return Err(Errno::EPERM);
            }
        }
    }

    pub async fn sys_tkill(&self, tid: usize, signum: usize) -> SysResult {
        debug!("sys_tkill @ tid: {}, signum: {}", tid, signum);
        let mut child = self.task.inner_map(|x| {
            x.threads
                .iter()
                .find(|x| match x.upgrade() {
                    Some(thread) => thread.task_id == tid,
                    None => false,
                })
                .map(|x| x.clone())
        });

        if tid == self.tid {
            child = Some(Arc::downgrade(&self.task));
        }

        match child {
            Some(child) => {
                let target_signal = SignalFlags::from_num(signum);
                let child_task = child.upgrade().unwrap();
                let mut child_tcb = child_task.tcb.write();
                if !child_tcb.signal.has_sig(target_signal.clone()) {
                    child_tcb.signal.add_signal(target_signal);
                } else {
                    if let Some(index) = target_signal.real_time_index() {
                        child_tcb.signal_queue[index] += 1;
                    }
                }
                // let signal = child
                //     .upgrade().unwrap()
                //     .tcb
                //     .write()
                //     .signal
                //     .add_signal(SignalFlags::from_usize(signum));
                Ok(0)
            }
            None => Err(Errno::ECHILD),
        }
    }

    pub async fn sys_sigreturn(&self) -> SysResult {
        debug!("sys_sigreturn @ ");
        Ok(0)
    }

    pub async fn sys_getrusage(&self, who: usize, usage_ptr: UserRef<Rusage>) -> SysResult {
        debug!("sys_getrusgae @ who: {}, usage_ptr: {}", who, usage_ptr);
        // let Rusage
        let rusage = usage_ptr.get_mut();

        let tms = self.task.inner_map(|inner| inner.tms);
        let stime = Time::from_raw(tms.stime as _);
        let utime = Time::from_raw(tms.utime as _);
        rusage.ru_stime = TimeVal {
            sec: stime.to_usec() / 1000_000,
            usec: stime.to_usec() % 1000_000,
        };
        rusage.ru_utime = TimeVal {
            sec: utime.to_usec() / 1000_000,
            usec: utime.to_usec() % 1000_000,
        };
        Ok(0)
    }

    pub fn sys_exit_group(&self, exit_code: usize) -> SysResult {
        debug!("sys_exit_group @ exit_code: {}", exit_code);
        // let children = user_task.pcb.lock().children.clone();
        // for ctask in children.iter().filter(|x| x.task_id != user_task.task_id) {
        //     ctask.exit(exit_code);
        // }
        self.task.exit(exit_code);
        Ok(0)
        // Err(LinuxError::EPERM)
    }

    pub async fn sys_kill(&self, pid: usize, signum: usize) -> SysResult {
        let signal = SignalFlags::from_num(signum);
        debug!(
            "[task {}] sys_kill @ pid: {}, signum: {:?}",
            self.tid, pid, signal
        );

        let user_task = match tid2task(pid) {
            Some(task) => task.downcast_arc::<UserTask>().map_err(|_| Errno::ESRCH),
            None => Err(Errno::ESRCH),
        }?;

        user_task.tcb.write().signal.add_signal(signal.clone());

        yield_now().await;

        Ok(0)
    }

    pub async fn sys_setsid(&self) -> SysResult {
        debug!("[task {}] sys_setsid", self.tid);
        let parent = self.task.parent.read().clone();

        if let Some(parent) = parent.upgrade() {
            parent.pcb.lock().children.retain(|x| x.task_id != self.tid);
            *self.task.parent.write() = Weak::<UserTask>::new();
        }
        Ok(0)
    }

    pub async fn sys_sched_getaffinity(
        &self,
        pid: usize,
        cpu_set_size: usize,
        mask: UserRef<u8>,
    ) -> SysResult {
        debug!(
            "[task {}] sys_sched_getaffinity @ pid: {}  cpu_set_size: {}, mask: {:#x?}",
            self.tid, pid, cpu_set_size, mask
        );
        // TODO:
        Ok(0)
    }
}
