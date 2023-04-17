use crate::syscall::c2rust_ref;
use crate::syscall::consts::from_vfs;
use crate::tasks::WaitPid;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{boxed::Box, sync::Arc};
use arch::{ppn_c, ContextOps, VirtPage, PAGE_SIZE};
use core::cmp;
use core::future::Future;
use executor::{current_task, yield_now, AsyncTask, MemType};
use frame_allocator::floor;
use fs::mount::open;
use log::debug;

use super::c2rust_buffer;
use super::{c2rust_list, c2rust_str, consts::LinuxError};

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

pub fn sys_exit(exit_code: usize) -> Result<usize, LinuxError> {
    debug!("sys_exit @ exit_code: {}", exit_code);
    current_task().as_user_task().unwrap().exit(exit_code);
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
    // let b = user_entry();
    // let task = UserTask::new(async {
    //     let t = unsafe { user_entry() };
    //     Pin::from(value)
    // });
    // let task = UserTask::new(unsafe { user_entry() }, Some(current_task()));
    let task = current_task().as_user_task().unwrap();
    exec_with_process(task.clone(), filename, args).await?;
    // thread::spawn(task.clone());
    Ok(0)
}

pub async fn sys_getpid() -> Result<usize, LinuxError> {
    Ok(current_task().get_task_id())
}

pub async fn exec_with_process<'a>(
    task: Arc<dyn AsyncTask>,
    path: &'a str,
    args_v: Vec<&'a str>,
) -> Result<Arc<dyn AsyncTask>, LinuxError> {
    let mut args = vec![path];
    args.extend(args_v.iter());
    let file = open(path).map_err(from_vfs)?;

    let mut buffer = vec![0u8; file.metadata().unwrap().size];
    file.read(&mut buffer).map_err(from_vfs)?;

    // 读取elf信息
    let elf = xmas_elf::ElfFile::new(&buffer).unwrap();
    let elf_header = elf.header;
    let magic = elf_header.pt1.magic;

    let entry_point = elf.header.pt2.entry_point() as usize;
    assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");

    // WARRNING: this convert async task to user task.
    let user_task = task.clone().as_user_task().unwrap();

    // // check if it is libc, dlopen, it needs recurit.
    // let header = elf
    //     .program_iter()
    //     .find(|ph| ph.get_type() == Ok(Type::Interp));
    // if let Some(header) = header {
    //     if let Ok(SegmentData::Undefined(_data)) = header.get_data(&elf) {
    //         let path = "libc.so";
    //         let mut new_args = vec![path];
    //         new_args.extend_from_slice(&args[..]);
    //         return exec_with_process(task, path, new_args).await;
    //     }
    // }

    // get heap_bottom, TODO: align 4096
    // brk is expanding the data section.
    let heap_bottom = elf.program_iter().fold(0, |acc, x| {
        if x.virtual_addr() + x.mem_size() > acc {
            x.virtual_addr() + x.mem_size()
        } else {
            acc
        }
    });

    // map stack
    user_task.frame_alloc(VirtPage::from_addr(0x7ffff000), MemType::Stack);

    user_task.inner_map(|mut inner| {
        inner.heap = heap_bottom as usize;
        inner.entry = entry_point;
        inner.cx.set_sp(0x8000_0000); // stack top;
        inner.cx.set_sepc(0x1000);
    });

    let args: Vec<usize> = args
        .into_iter()
        .rev()
        .map(|x| user_task.push_str(x))
        .collect();
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
            let virt_addr = ph.virtual_addr() as usize;

            let page_count = floor(mem_size, PAGE_SIZE);
            let ppn_start = user_task.frame_alloc_much(
                VirtPage::from_addr(virt_addr),
                MemType::CodeSection,
                page_count,
            );

            let page_space = unsafe {
                core::slice::from_raw_parts_mut(ppn_c(ppn_start).to_addr() as _, file_size)
            };
            page_space.copy_from_slice(&buffer[offset..offset + file_size]);
        });

    Ok(task)
}

pub async fn sys_clone(
    flags: usize, // 复制 标志位
    stack: usize, // 指定新的栈，可以为 0, 0 不处理
    ptid: usize,  // 父线程 id
    tls: usize,   // TLS线程本地存储描述符
    ctid: usize,  // 子线程 id
) -> Result<usize, LinuxError> {
    debug!(
        "sys_clone @ flags: {:#x}, stack: {:#x}, ptid: {}, tls: {:#x}, ctid: {}",
        flags, stack, ptid, tls, ctid
    );
    let curr_task = current_task().as_user_task().unwrap();
    let new_task = curr_task.fork(unsafe { user_entry() });
    if stack != 0 {
        new_task.inner.lock().cx.set_sp(stack);
    }
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
    let child_task = WaitPid(curr_task.clone(), pid).await;
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

pub async fn sys_getppid() -> Result<usize, LinuxError> {
    debug!("sys_getppid @ ");
    current_task()
        .as_user_task()
        .unwrap()
        .parent
        .as_ref()
        .map(|x| x.get_task_id())
        .ok_or(LinuxError::EPERM)
}
