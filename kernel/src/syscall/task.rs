use crate::syscall::consts::{from_vfs, CloneFlags, Rusage};
use crate::syscall::time::WaitUntilsec;
use crate::tasks::elf::{init_task_stack, ElfExtra};
use crate::tasks::user::entry::user_entry;
use crate::tasks::{futex_requeue, futex_wake, WaitFutex, WaitPid};
use alloc::string::String;
use alloc::sync::Weak;
use alloc::vec::Vec;
use alloc::{boxed::Box, sync::Arc};
use arch::{paddr_c, ppn_c, time_to_usec, ContextOps, PhysAddr, VirtAddr, VirtPage, PAGE_SIZE};
use async_recursion::async_recursion;
use core::cmp;
use executor::{
    current_task, current_user_task, select, yield_now, AsyncTask, MemType, UserTask, TASK_QUEUE,
};
use frame_allocator::{ceil_div, frame_alloc_much};
use fs::mount::open;
use fs::TimeSpec;
use hal::{current_nsec, TimeVal};
use log::{debug, warn};
use signal::SignalFlags;
use xmas_elf::program::{SegmentData, Type};

use super::consts::{FutexFlags, LinuxError, UserRef};

pub async fn sys_chdir(path_ptr: UserRef<i8>) -> Result<usize, LinuxError> {
    let path = path_ptr.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    debug!("sys_chdir @ path: {}", path);
    // check folder exists
    let dir = open(path).map_err(from_vfs)?;
    match dir.metadata().unwrap().file_type {
        fs::FileType::Directory => {
            let user_task = current_task().as_user_task().unwrap();
            let mut inner = user_task.pcb.lock();
            match path.starts_with("/") {
                true => inner.curr_dir = String::from(path),
                false => inner.curr_dir += path,
            }
            Ok(0)
        }
        _ => Err(LinuxError::ENOTDIR),
    }
}

pub async fn sys_getcwd(buf_ptr: UserRef<u8>, size: usize) -> Result<usize, LinuxError> {
    debug!("sys_getcwd @ buffer_ptr{} size: {}", buf_ptr, size);
    let buffer = buf_ptr.slice_mut_with_len(size);
    let curr_path = current_user_task().pcb.lock().curr_dir.clone();
    let bytes = curr_path.as_bytes();
    let len = cmp::min(bytes.len(), size);
    buffer[..len].copy_from_slice(&bytes[..len]);
    Ok(buf_ptr.into())
}

pub fn sys_exit(exit_code: isize) -> Result<usize, LinuxError> {
    debug!(
        "sys_exit @ exit_code: {}  task_id: {}",
        exit_code,
        current_task().get_task_id()
    );
    // current_task().as_user_task().unwrap().exit(exit_code as _);
    current_task()
        .as_user_task()
        .unwrap()
        .thread_exit(exit_code as _);
    Ok(0)
}

pub async fn sys_execve(
    filename: UserRef<i8>,      // *mut i8
    args: UserRef<UserRef<i8>>, // *mut *mut i8
    envp: UserRef<UserRef<i8>>, // *mut *mut i8
) -> Result<usize, LinuxError> {
    // TODO: use map_err insteads of unwrap and unsafe code.
    let filename = filename.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    let args = args
        .slice_until_valid(|x| x.is_valid())
        .into_iter()
        .map(|x| x.get_cstr().unwrap())
        .collect();
    let envp: Vec<&str> = envp
        .slice_until_valid(|x| x.is_valid())
        .into_iter()
        .map(|x| x.get_cstr().unwrap())
        .collect();
    debug!(
        "sys_execve @ filename: {} args: {:?}: envp: {:?}",
        filename, args, envp
    );

    let task = current_task().as_user_task().unwrap();
    // clear memory map
    // TODO: solve memory conflict
    // task.pcb.lock().memset.retain(|x| x.mtype == MemType::PTE);
    exec_with_process(task.clone(), filename, args).await?;
    task.before_run();
    Ok(0)
}

#[async_recursion(?Send)]
pub async fn exec_with_process<'a>(
    task: Arc<dyn AsyncTask>,
    path: &'a str,
    args: Vec<&'a str>,
) -> Result<Arc<UserTask>, LinuxError> {
    // copy args, avoid free before pushing.
    let args: Vec<String> = args.into_iter().map(|x| String::from(x)).collect();
    debug!("exec: {:?}", args);

    if path == "/bin/true" {
        let ctask = task.as_user_task().unwrap();
        ctask.exit(0);
        return Ok(ctask);
    }

    let file = open(path).map_err(from_vfs)?;
    let file_size = file.metadata().unwrap().size;
    let frame_ppn = frame_alloc_much(ceil_div(file_size, PAGE_SIZE));
    let buffer = PhysAddr::from(frame_ppn.as_ref().unwrap()[0].0).slice_mut_with_len(file_size);
    let rsize = file.read(buffer).map_err(from_vfs)?;

    assert_eq!(rsize, file_size);

    // 读取elf信息
    let elf = if let Ok(elf) = xmas_elf::ElfFile::new(&buffer) {
        elf
    } else {
        let mut new_args = vec!["busybox", "sh"];
        args.iter().for_each(|x| new_args.push(x));
        return exec_with_process(task, "busybox", new_args).await;
    };
    let elf_header = elf.header;

    let entry_point = elf.header.pt2.entry_point() as usize;
    // this assert ensures that the file is elf file.
    assert_eq!(
        elf_header.pt1.magic,
        [0x7f, 0x45, 0x4c, 0x46],
        "invalid elf!"
    );
    // WARRNING: this convert async task to user task.
    let user_task = task.clone().as_user_task().unwrap();

    // check if it is libc, dlopen, it needs recurit.
    let header = elf
        .program_iter()
        .find(|ph| ph.get_type() == Ok(Type::Interp));
    if let Some(header) = header {
        drop(frame_ppn);
        if let Ok(SegmentData::Undefined(_data)) = header.get_data(&elf) {
            let lib_path = "libc.so";
            let mut new_args = vec![lib_path, path];
            args[1..].iter().for_each(|x| new_args.push(x));
            return exec_with_process(task, lib_path, new_args).await;
        }
    }

    // get heap_bottom, TODO: align 4096
    // brk is expanding the data section.
    let heap_bottom = elf.program_iter().fold(0, |acc, x| {
        if x.virtual_addr() + x.mem_size() > acc {
            x.virtual_addr() + x.mem_size()
        } else {
            acc
        }
    });

    let tls = elf
        .program_iter()
        .find(|x| x.get_type().unwrap() == xmas_elf::program::Type::Tls)
        .map(|ph| ph.virtual_addr())
        .unwrap_or(0);

    let base = 0x20000000;

    let (base, relocated_arr) = match elf.relocate(base) {
        Ok(arr) => (base, arr),
        Err(_) => (0, vec![]),
    };

    init_task_stack(
        user_task.clone(),
        args,
        base,
        path,
        entry_point,
        elf_header.pt2.ph_count() as usize,
        elf_header.pt2.ph_entry_size() as usize,
        elf.get_ph_addr().unwrap_or(0) as usize,
        heap_bottom as usize,
        tls as usize,
    );

    // map sections.
    elf.program_iter()
        .filter(|x| x.get_type().unwrap() == xmas_elf::program::Type::Load)
        .for_each(|ph| {
            let file_size = ph.file_size() as usize;
            let mem_size = ph.mem_size() as usize;
            let offset = ph.offset() as usize;
            let virt_addr = base + ph.virtual_addr() as usize;
            let vpn = virt_addr / PAGE_SIZE;

            let page_count = ceil_div(virt_addr + mem_size, PAGE_SIZE) - vpn;
            let ppn_start = user_task.frame_alloc(
                VirtPage::from_addr(virt_addr),
                MemType::CodeSection,
                page_count,
            );

            let page_space = unsafe {
                core::slice::from_raw_parts_mut(
                    (ppn_c(ppn_start).to_addr() + virt_addr % PAGE_SIZE) as _,
                    file_size,
                )
            };
            page_space.copy_from_slice(&buffer[offset..offset + file_size]);
        });

    if base > 0 {
        relocated_arr.into_iter().for_each(|(addr, value)| unsafe {
            (paddr_c(user_task.page_table.virt_to_phys(VirtAddr::from(addr))).addr() as *mut usize)
                .write(value);
        })
    }
    Ok(user_task)
}

pub async fn sys_clone(
    flags: usize,       // 复制 标志位
    stack: usize,       // 指定新的栈，可以为 0, 0 不处理
    ptid: UserRef<u32>, // 父线程 id
    tls: usize,         // TLS线程本地存储描述符
    ctid: UserRef<u32>, // 子线程 id
) -> Result<usize, LinuxError> {
    let sig = flags & 0xff;
    let curr_task = current_task().as_user_task().unwrap();
    debug!(
        "[task {}] sys_clone @ flags: {:#x}, stack: {:#x}, ptid: {}, tls: {:#x}, ctid: {}",
        curr_task.get_task_id(),
        flags,
        stack,
        ptid,
        tls,
        ctid
    );
    let flags = CloneFlags::from_bits_truncate(flags);
    debug!(
        "[task {}] sys_clone @ flags: {:?}, stack: {:#x}, ptid: {}, tls: {:#x}, ctid: {}",
        curr_task.get_task_id(),
        flags,
        stack,
        ptid,
        tls,
        ctid
    );

    let new_task = match flags.contains(CloneFlags::CLONE_THREAD) {
        true => curr_task.clone().thread_clone(user_entry()),
        // false => curr_task.clone().fork(unsafe { user_entry() }),
        // use cow(Copy On Write) to save memory.
        false => curr_task.clone().cow_fork(user_entry()),
    };

    let clear_child_tid = flags
        .contains(CloneFlags::CLONE_CHILD_CLEARTID)
        .then_some(ctid)
        .unwrap_or(UserRef::from(0));

    let mut new_tcb = new_task.tcb.write();

    new_tcb.clear_child_tid = clear_child_tid.addr();

    if stack != 0 {
        new_tcb.cx.set_sp(stack);
    }
    // set tls.
    if flags.contains(CloneFlags::CLONE_SETTLS) {
        new_tcb.cx.set_tls(tls);
    }
    if flags.contains(CloneFlags::CLONE_PARENT_SETTID) {
        *ptid.get_mut() = new_task.get_task_id() as _;
    }
    if flags.contains(CloneFlags::CLONE_CHILD_SETTID) {
        *ctid.get_mut() = new_task.get_task_id() as _;
    }
    new_tcb.exit_signal = sig as u8;
    drop(new_tcb);
    yield_now().await;
    Ok(new_task.task_id)
}

pub async fn sys_wait4(
    pid: isize,           // 指定进程ID，可为-1等待任何子进程；
    status: UserRef<i32>, // 接收状态的指针；
    options: usize,       // WNOHANG，WUNTRACED，WCONTINUED；
) -> Result<usize, LinuxError> {
    let curr_task = current_task().as_user_task().unwrap();
    debug!(
        "[task {}] sys_wait4 @ pid: {}, status: {}, options: {}",
        curr_task.get_task_id(),
        pid,
        status,
        options
    );

    // return LinuxError::ECHILD if there has no child process.
    if curr_task.inner_map(|inner| inner.children.len()) == 0 {
        return Err(LinuxError::ECHILD);
    }

    if pid != -1 {
        curr_task
            .inner_map(|inner| {
                inner
                    .children
                    .iter()
                    .find(|x| x.task_id == pid as usize)
                    .map(|x| x.clone())
            })
            .ok_or(LinuxError::ECHILD)?;
    }
    if options == 0 || options == 2 || options == 3 {
        debug!(
            "children:{:?}",
            curr_task.pcb.lock().children.iter().count()
        );
        let child_task = WaitPid(curr_task.clone(), pid).await?;

        debug!(
            "wait ok: {}  waiter: {}",
            child_task.get_task_id(),
            curr_task.get_task_id()
        );
        curr_task
            .pcb
            .lock()
            .children
            .drain_filter(|x| x.task_id == child_task.get_task_id());
        debug!("wait pid: {}", child_task.exit_code().unwrap());

        if status.is_valid() {
            *status.get_mut() = (child_task.exit_code().unwrap() as i32) << 8;
        }
        Ok(child_task.task_id)
    } else if options == 1 {
        let child_task = curr_task
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
                curr_task
                    .pcb
                    .lock()
                    .children
                    .drain_filter(|x| x.task_id == child_task.task_id);
                if status.is_valid() {
                    *status.get_mut() = (t1 as i32) << 8;
                }
                // TIPS: This is a small change.
                Ok(child_task.get_task_id())
                // Ok(0)
            }
            None => Ok(0),
        }
    } else {
        warn!("wait4 unsupported options: {}", options);
        Err(LinuxError::EPERM)
    }
}

pub async fn sys_sched_yield() -> Result<usize, LinuxError> {
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
pub async fn sys_set_tid_address(tid_ptr: usize) -> Result<usize, LinuxError> {
    // information source: https://www.onitroad.com/jc/linux/man-pages/linux/man2/set_tid_address.2.html

    debug!("sys_set_tid_address @ tid_ptr: {:#x}", tid_ptr);
    current_user_task().tcb.write().clear_child_tid = tid_ptr;
    Ok(current_task().get_task_id())
}

/// sys_getpid() 获取进程 id
pub async fn sys_getpid() -> Result<usize, LinuxError> {
    Ok(current_user_task().process_id)
}

/// sys_getppid() 获取父进程 id
pub async fn sys_getppid() -> Result<usize, LinuxError> {
    debug!("sys_getppid @ ");
    current_user_task()
        .parent
        .read()
        .upgrade()
        .map(|x| x.get_task_id())
        .ok_or(LinuxError::EPERM)
}

/// sys_gettid() 获取线程 id.
/// need to write correct clone and thread_clone for pthread.
pub async fn sys_gettid() -> Result<usize, LinuxError> {
    debug!("sys_gettid @ ");
    Ok(current_task().get_task_id())
}

pub async fn sys_futex(
    uaddr_ptr: UserRef<i32>,
    op: usize,
    value: usize,
    value2: usize,
    uaddr2: usize,
    value3: usize,
) -> Result<usize, LinuxError> {
    let op = if op >= 0x80 { op - 0x80 } else { op };
    let user_task = current_user_task();
    debug!(
        "[task {}] sys_futex @ uaddr: {} op: {} value: {:#x}, value2: {:#x}, uaddr2: {:#x} , value3: {:#x}",
        user_task.get_task_id(), uaddr_ptr, op, value, value2, uaddr2, value3
    );
    let uaddr = uaddr_ptr.get_mut();
    let flags = FutexFlags::try_from(op).map_err(|_| LinuxError::EINVAL)?;
    debug!(
        "sys_futex @ uaddr: {} flags: {:?} value: {}",
        uaddr, flags, value
    );

    match flags {
        FutexFlags::Wait => {
            if *uaddr == value as i32 {
                let futex_table = user_task.pcb.lock().futex_table.clone();
                let mut table = futex_table.lock();
                match table.get_mut(&uaddr_ptr.addr()) {
                    Some(t) => t.push(user_task.task_id),
                    None => {
                        table.insert(uaddr_ptr.addr(), vec![user_task.task_id]);
                    }
                }
                drop(table);
                let wait_func = WaitFutex(futex_table.clone(), user_task.task_id);
                if value2 != 0 {
                    let timeout = UserRef::<TimeSpec>::from(value2).get_mut();
                    match select(wait_func, WaitUntilsec(current_nsec() + timeout.to_nsec())).await
                    {
                        executor::Either::Left((res, _)) => res,
                        executor::Either::Right(_) => Err(LinuxError::ETIMEDOUT),
                    }
                } else {
                    wait_func.await
                }
                // wait_func.await
            } else {
                Err(LinuxError::EAGAIN)
            }
        }
        FutexFlags::Wake => {
            let futex_table = user_task.pcb.lock().futex_table.clone();
            let count = futex_wake(futex_table, uaddr_ptr.addr(), value);
            yield_now().await;
            Ok(count)
        }
        FutexFlags::Requeue => {
            let futex_table = user_task.pcb.lock().futex_table.clone();
            Ok(futex_requeue(
                futex_table,
                uaddr_ptr.addr(),
                value,
                uaddr2,
                value2,
            ))
        }
        _ => {
            return Err(LinuxError::EPERM);
        }
    }
}

pub async fn sys_tkill(tid: usize, signum: usize) -> Result<usize, LinuxError> {
    debug!("sys_tkill @ tid: {}, signum: {}", tid, signum);
    let task = current_user_task();
    let mut child = task.inner_map(|x| {
        x.threads
            .iter()
            .find(|x| match x.upgrade() {
                Some(thread) => thread.task_id == tid,
                None => false,
            })
            .map(|x| x.clone())
    });

    if tid == task.task_id {
        child = Some(Arc::downgrade(&task));
    }

    match child {
        Some(child) => {
            let target_signal = SignalFlags::from_usize(signum);
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
        None => Err(LinuxError::ECHILD),
    }
}

pub async fn sys_sigreturn() -> Result<usize, LinuxError> {
    debug!("sys_sigreturn @ ");
    Ok(0)
}

pub async fn sys_getrusage(who: usize, usage_ptr: UserRef<Rusage>) -> Result<usize, LinuxError> {
    debug!("sys_getrusgae @ who: {}, usage_ptr: {}", who, usage_ptr);
    // let Rusage
    let rusage = usage_ptr.get_mut();

    let tms = current_user_task().inner_map(|inner| inner.tms);
    let stime = time_to_usec(tms.stime as _);
    let utime = time_to_usec(tms.utime as _);
    rusage.ru_stime = TimeVal {
        sec: stime / 1000_000,
        usec: stime % 1000_000,
    };
    rusage.ru_utime = TimeVal {
        sec: utime / 1000_000,
        usec: utime % 1000_000,
    };
    Ok(0)
}

pub fn sys_exit_group(exit_code: usize) -> Result<usize, LinuxError> {
    debug!("sys_exit_group @ exit_code: {}", exit_code);
    let user_task = current_user_task();
    // let children = user_task.pcb.lock().children.clone();
    // for ctask in children.iter().filter(|x| x.task_id != user_task.task_id) {
    //     ctask.exit(exit_code);
    // }
    user_task.exit(exit_code);
    Ok(0)
    // Err(LinuxError::EPERM)
}

pub async fn sys_kill(pid: usize, signum: usize) -> Result<usize, LinuxError> {
    let signal = SignalFlags::from_usize(signum);
    let curr_task = current_task();
    debug!(
        "[task {}] sys_kill @ pid: {}, signum: {:?}",
        curr_task.get_task_id(),
        pid,
        signal
    );

    let user_task = match pid == current_task().get_task_id() {
        true => Some(curr_task.clone().as_user_task().unwrap()),
        false => TASK_QUEUE
            .lock()
            .iter()
            .find(|x| x.get_task_id() == pid)
            .map(|x| x.clone())
            .ok_or(LinuxError::ESRCH)?
            .as_user_task(),
    };

    let user_task = match user_task {
        Some(t) => t,
        None => return Err(LinuxError::ESRCH),
    };

    user_task.tcb.write().signal.add_signal(signal.clone());

    yield_now().await;

    Ok(0)
}

pub async fn sys_setsid() -> Result<usize, LinuxError> {
    let user_task = current_user_task();
    debug!("[task {}] sys_setsid", user_task.get_task_id());
    let parent = user_task.parent.read().clone();

    if let Some(parent) = parent.upgrade() {
        if let Some(parent) = parent.as_user_task() {
            parent
                .pcb
                .lock()
                .children
                .retain(|x| x.get_task_id() != user_task.get_task_id());
        }
        *user_task.parent.write() = Weak::<UserTask>::new();
    }
    Ok(0)
}
