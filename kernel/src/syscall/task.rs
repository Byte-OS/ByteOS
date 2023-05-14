use crate::syscall::consts::{elf, from_vfs, CloneFlags, Rusage};
use crate::syscall::func::{c2rust_buffer, c2rust_list, c2rust_ref, c2rust_str};
use crate::syscall::time::{current_nsec, TimeVal, WaitUntilsec};
use crate::tasks::elf::ElfExtra;
use crate::tasks::{futex_requeue, futex_wake, WaitFutex, WaitPid};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{boxed::Box, sync::Arc};
use arch::{paddr_c, ppn_c, time_to_usec, ContextOps, VirtAddr, VirtPage, PAGE_SIZE};
use core::cmp;
use core::future::Future;
use executor::{
    current_task, current_user_task, select, yield_now, AsyncTask, MemType, TASK_QUEUE,
};
use frame_allocator::{ceil_div, frame_alloc_much};
use fs::mount::open;
use fs::TimeSpec;
use log::debug;
use signal::SignalFlags;
use xmas_elf::program::{SegmentData, Type};

use super::consts::{FutexFlags, LinuxError};

extern "Rust" {
    fn user_entry() -> Box<dyn Future<Output = ()> + Send + Sync>;
}

pub async fn sys_chdir(path_ptr: usize) -> Result<usize, LinuxError> {
    let path = c2rust_str(path_ptr as *mut i8);
    debug!("sys_chdir @ path: {}", path);
    // check folder exists
    let dir = open(path).map_err(from_vfs)?;
    match dir.metadata().unwrap().file_type {
        fs::FileType::Directory => {
            let user_task = current_task().as_user_task().unwrap();
            let mut inner = user_task.inner.lock();
            match path.starts_with("/") {
                true => inner.curr_dir = String::from(path),
                false => inner.curr_dir += path,
            }
            Ok(0)
        }
        _ => Err(LinuxError::ENOTDIR),
    }
}

pub async fn sys_getcwd(buf_ptr: usize, size: usize) -> Result<usize, LinuxError> {
    debug!("sys_getcwd @ buffer_ptr{:#x} size: {}", buf_ptr, size);
    let buffer = c2rust_buffer(buf_ptr as *mut u8, size);
    let curr_path = current_task()
        .as_user_task()
        .unwrap()
        .inner
        .lock()
        .curr_dir
        .clone();
    let bytes = curr_path.as_bytes();
    let len = cmp::min(bytes.len(), size);
    buffer[..len].copy_from_slice(&bytes[..len]);
    Ok(buf_ptr)
}

pub fn sys_exit(exit_code: isize) -> Result<usize, LinuxError> {
    debug!("sys_exit @ exit_code: {}", exit_code);
    current_task().as_user_task().unwrap().exit(exit_code as _);
    Ok(0)
}

pub async fn sys_execve(
    filename: usize, // *mut i8
    args: usize,     // *mut *mut i8
    envp: usize,     // *mut *mut i8
) -> Result<usize, LinuxError> {
    // TODO: use map_err insteads of unwrap and unsafe code.
    let filename = c2rust_str(filename as *mut i8);
    let args: Vec<&str> = c2rust_list(args as *mut *mut i8, |x| !x.is_null())
        .into_iter()
        .map(|x| c2rust_str(*x))
        .collect();
    let envp: Vec<&str> = c2rust_list(envp as *mut *mut i8, |x| !x.is_null())
        .into_iter()
        .map(|x| c2rust_str(*x))
        .collect();

    debug!(
        "sys_execve @ filename: {} args: {:?}: envp: {:?}",
        filename, args, envp
    );

    let task = current_task().as_user_task().unwrap();
    exec_with_process(task.clone(), filename, args)?;
    Ok(0)
}

pub fn exec_with_process<'a>(
    task: Arc<dyn AsyncTask>,
    path: &'a str,
    args: Vec<&'a str>,
) -> Result<Arc<dyn AsyncTask>, LinuxError> {
    // copy args, avoid free before pushing.
    let args: Vec<String> = args.into_iter().map(|x| String::from(x)).collect();
    debug!("exec: {:?}", args);

    let file = open(path).map_err(from_vfs)?;
    let file_size = file.metadata().unwrap().size;
    let frame_ppn = frame_alloc_much(ceil_div(file_size, PAGE_SIZE));
    let mut buffer = c2rust_buffer(
        ppn_c(frame_ppn.as_ref().unwrap()[0].0).to_addr() as *mut u8,
        file_size,
    );
    let rsize = file.read(&mut buffer).map_err(from_vfs)?;

    assert_eq!(rsize, file_size);

    // 读取elf信息
    let elf = if let Ok(elf) = xmas_elf::ElfFile::new(&buffer) {
        elf
    } else {
        let mut new_args = vec!["busybox", "sh"];
        args.iter().for_each(|x| new_args.push(x));
        return exec_with_process(task, "busybox", new_args);
    };
    let elf_header = elf.header;

    let entry_point = elf.header.pt2.entry_point() as usize;
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
            return exec_with_process(task, lib_path, new_args);
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

    // map stack
    user_task.frame_alloc_much(VirtPage::from_addr(0x7fffe000), MemType::Stack, 2);
    debug!("entry: {:#x}", base + entry_point);
    user_task.inner_map(|inner| {
        inner.heap = heap_bottom as usize;
        inner.entry = base + entry_point;
        inner.cx.clear();
        inner.cx.set_sp(0x8000_0000); // stack top;
        inner.cx.set_sepc(base + entry_point);
        inner.cx.set_tls(tls as usize);
    });

    // push stack
    let envp = vec![
        "LD_LIBRARY_PATH=/",
        "PS1=\x1b[1m\x1b[32mByteOS\x1b[0m:\x1b[1m\x1b[34m\\w\x1b[0m\\$ \0",
        "PATH=/:/bin:/usr/bin",
    ];
    let envp: Vec<usize> = envp
        .into_iter()
        .rev()
        .map(|x| user_task.push_str(x))
        .collect();
    let args: Vec<usize> = args
        .into_iter()
        .rev()
        .map(|x| user_task.push_str(&x))
        .collect();

    let random_ptr = user_task.push_arr(&[0u8; 16]);
    let mut auxv = BTreeMap::new();
    auxv.insert(elf::AT_PLATFORM, user_task.push_str("riscv"));
    auxv.insert(elf::AT_EXECFN, user_task.push_str(path));
    auxv.insert(elf::AT_PHNUM, elf_header.pt2.ph_count() as usize);
    auxv.insert(elf::AT_PAGESZ, PAGE_SIZE);
    auxv.insert(elf::AT_ENTRY, base + entry_point);
    auxv.insert(elf::AT_PHENT, elf_header.pt2.ph_entry_size() as usize);
    auxv.insert(elf::AT_PHDR, base + elf.get_ph_addr().unwrap_or(0) as usize);
    auxv.insert(elf::AT_GID, 0);
    auxv.insert(elf::AT_EGID, 0);
    auxv.insert(elf::AT_UID, 0);
    auxv.insert(elf::AT_EUID, 0);
    auxv.insert(elf::AT_SECURE, 0);
    auxv.insert(elf::AT_RANDOM, random_ptr);

    // auxv top
    user_task.push_num(0);
    // TODO: push auxv
    auxv.iter().for_each(|(key, v)| {
        user_task.push_num(*v);
        user_task.push_num(*key);
    });

    user_task.push_num(0);
    envp.iter().for_each(|x| {
        user_task.push_num(*x);
    });
    user_task.push_num(0);
    args.iter().for_each(|x| {
        user_task.push_num(*x);
    });
    user_task.push_num(args.len());

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
            let ppn_start = user_task.frame_alloc_much(
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
            debug!("addr: {:#X} value: {:#x}", addr, value);
            (paddr_c(user_task.page_table.virt_to_phys(VirtAddr::from(addr))).addr() as *mut usize)
                .write(value);
        })
    }

    Ok(task)
}

pub async fn sys_clone(
    flags: usize, // 复制 标志位
    stack: usize, // 指定新的栈，可以为 0, 0 不处理
    ptid: usize,  // 父线程 id
    tls: usize,   // TLS线程本地存储描述符
    ctid: usize,  // 子线程 id
) -> Result<usize, LinuxError> {
    let flags = CloneFlags::from_bits_truncate(flags);
    debug!(
        "sys_clone @ flags: {:?}, stack: {:#x}, ptid: {:#x}, tls: {:#x}, ctid: {:#x}",
        flags, stack, ptid, tls, ctid
    );
    let curr_task = current_task().as_user_task().unwrap();

    let new_task = match flags.contains(CloneFlags::CLONE_THREAD) {
        true => curr_task.clone().thread_clone(unsafe { user_entry() }),
        false => curr_task.clone().fork(unsafe { user_entry() }),
    };

    let clear_child_tid = flags
        .contains(CloneFlags::CLONE_CHILD_CLEARTID)
        .then_some(ctid)
        .unwrap_or(0);

    new_task.inner_map(|inner| {
        inner.clear_child_tid = clear_child_tid;
        // inner.set_child_tid = set_child_tid;
    });

    if stack != 0 {
        new_task.inner.lock().cx.set_sp(stack);
    }
    // set tls.
    if flags.contains(CloneFlags::CLONE_SETTLS) {
        new_task.inner.lock().cx.set_tls(tls);
    }
    if flags.contains(CloneFlags::CLONE_PARENT_SETTID) {
        *c2rust_ref(ptid as *mut u32) = new_task.get_task_id() as _;
    }
    if flags.contains(CloneFlags::CLONE_CHILD_SETTID) {
        *c2rust_ref(ctid as *mut u32) = new_task.get_task_id() as _;
    }
    // yield_now().await;
    Ok(new_task.task_id)
}

pub async fn sys_wait4(
    pid: isize,     // 指定进程ID，可为-1等待任何子进程；
    status: usize,  // 接收状态的指针；
    options: usize, // WNOHANG，WUNTRACED，WCONTINUED；
) -> Result<usize, LinuxError> {
    debug!(
        "sys_wait4 @ pid: {}, status: {:#x}, options: {}",
        pid, status, options
    );
    let curr_task = current_task().as_user_task().unwrap();

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

    let child_task = WaitPid(curr_task.clone(), pid).await;
    debug!("wait ok: {}", child_task.get_task_id());
    curr_task
        .inner
        .lock()
        .children
        .drain_filter(|x| x.task_id == child_task.get_task_id());
    debug!("wait pid: {}", child_task.exit_code().unwrap());

    if status != 0 {
        let status_ref = c2rust_ref(status as *mut i32);
        *status_ref = (child_task.exit_code().unwrap() as i32) << 8;
        debug!("waitpid acc: {}", *status_ref);
    }

    Ok(child_task.task_id)
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
    current_task()
        .as_user_task()
        .unwrap()
        .inner_map(|inner| inner.clear_child_tid = tid_ptr);
    Ok(current_task().get_task_id())
}

/// sys_getpid() 获取进程 id
pub async fn sys_getpid() -> Result<usize, LinuxError> {
    Ok(current_task().get_task_id())
}

/// sys_getppid() 获取父进程 id
pub async fn sys_getppid() -> Result<usize, LinuxError> {
    debug!("sys_getppid @ ");
    current_user_task()
        .parent
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
    uaddr_ptr: usize,
    op: usize,
    value: usize,
    value2: usize,
    uaddr2: usize,
    value3: usize,
) -> Result<usize, LinuxError> {
    let op = if op >= 0x80 { op - 0x80 } else { op };
    debug!(
        "sys_futex @ uaddr: {:#x} op: {} value: {:#x}, value2: {:#x}, uaddr2: {:#x} , value3: {:#x}",
        uaddr_ptr, op, value, value2, uaddr2, value3
    );
    let uaddr = c2rust_ref(uaddr_ptr as *mut i32);
    let flags = FutexFlags::try_from(op).map_err(|_| LinuxError::EINVAL)?;
    let user_task = current_user_task();
    debug!(
        "sys_futex @ uaddr: {} flags: {:?} value: {}",
        uaddr, flags, value
    );

    match flags {
        FutexFlags::Wait => {
            if *uaddr == value as i32 {
                let futex_table = user_task.inner.lock().futex_table.clone();
                let mut table = futex_table.lock();
                match table.get_mut(&uaddr_ptr) {
                    Some(t) => t.push(user_task.task_id),
                    None => {
                        table.insert(uaddr_ptr, vec![user_task.task_id]);
                    }
                }
                drop(table);
                let wait_func = WaitFutex(futex_table.clone(), user_task.task_id);
                if value2 != 0 {
                    let timeout = c2rust_ref(value2 as *mut TimeSpec);
                    debug!("timeout: {:?}", timeout);
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
            let futex_table = user_task.inner.lock().futex_table.clone();
            let count = futex_wake(futex_table, uaddr_ptr, value);
            yield_now().await;
            Ok(count)
        }
        FutexFlags::Requeue => {
            let futex_table = user_task.inner.lock().futex_table.clone();
            Ok(futex_requeue(futex_table, uaddr_ptr, value, uaddr2, value2))
        }
        _ => {
            return Err(LinuxError::EPERM);
        }
    }
}

pub async fn sys_tkill(tid: usize, signum: usize) -> Result<usize, LinuxError> {
    debug!("sys_tkill @ tid: {}, signum: {}", tid, signum);
    let task = current_user_task();
    let child = task.inner_map(|x| {
        x.children
            .iter()
            .find(|x| x.task_id == tid)
            .map(|x| x.clone())
    });
    match child {
        Some(child) => {
            child.inner_map(|x| {
                x.signal.add_signal(SignalFlags::from_usize(signum));
            });
            Ok(0)
        }
        None => Err(LinuxError::ECHILD),
    }
}

pub async fn sys_sigreturn() -> Result<usize, LinuxError> {
    debug!("sys_sigreturn @ ");
    Err(LinuxError::CONTROLFLOWBREAK)
}

pub async fn sys_getrusage(who: usize, usage_ptr: usize) -> Result<usize, LinuxError> {
    debug!("sys_getrusgae @ who: {}, usage_ptr: {:#x}", who, usage_ptr);
    let rusage = c2rust_ref(usage_ptr as *mut Rusage);

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

// pub fn sys_exit_group(exit_code: usize) -> Result<(), RuntimeError> {
//     let inner = self.inner.borrow_mut();
//     let mut process = inner.process.borrow_mut();
//     debug!("exit pid: {}", self.pid);
//     process.exit(exit_code);
//     match &process.parent {
//         Some(parent) => {
//             let parent = parent.upgrade().unwrap();
//             let parent = parent.borrow();
//             remove_vfork_wait(parent.pid);

//             // let end: UserAddr<TimeSpec> = 0x10bb78.into();
//             // let start: UserAddr<TimeSpec> = 0x10bad0.into();

//             // println!("start: {:?}   end: {:?}",start.transfer(), end.transfer());

//             // let target_end: UserAddr<TimeSpec> = parent.pmm.get_phys_addr(0x10bb78usize.into())?.0.into();
//             // let target_start: UserAddr<TimeSpec> = parent.pmm.get_phys_addr(0x10bad0usize.into())?.0.into();
//             // *target_start.transfer() = *start.transfer();
//             // *target_end.transfer() = *end.transfer();

//             // let task = parent.tasks[0].clone().upgrade().unwrap();
//             // drop(parent);
//             // // 处理signal 17 SIGCHLD
//             // task.signal(17);
//         }
//         None => {}
//     }
//     debug!("剩余页表: {}", get_free_page_num());
//     debug!("exit_code: {:#x}", exit_code);
//     Err(RuntimeError::ChangeTask)
// }

pub async fn sys_kill(pid: usize, signum: usize) -> Result<usize, LinuxError> {
    let signal = SignalFlags::from_usize(signum);
    debug!("sys_kill @ pid: {}, signum: {:?}", pid, signal);
    debug!("current_user_task: {}", current_user_task().task_id);

    let user_task = TASK_QUEUE
        .lock()
        .iter()
        .find(|x| x.get_task_id() == pid)
        .map(|x| x.clone())
        .ok_or(LinuxError::ESRCH)?
        .as_user_task();

    let user_task = match user_task {
        Some(t) => t,
        None => return Err(LinuxError::ESRCH),
    };

    match signal {
        SignalFlags::SIGKILL => {
            user_task.exit_with_signal(signal.num());
        }
        _ => {
            user_task.inner_map(|inner| inner.signal.add_signal(signal.clone()));
        }
    }

    yield_now().await;

    Ok(0)
}
