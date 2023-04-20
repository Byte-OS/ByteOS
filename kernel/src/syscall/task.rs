use crate::syscall::c2rust_ref;
use crate::syscall::consts::{from_vfs, elf};
use crate::tasks::WaitPid;
use crate::tasks::elf::ElfExtra;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{boxed::Box, sync::Arc};
use arch::{ppn_c, ContextOps, VirtPage, PAGE_SIZE};
use core::cmp;
use core::future::Future;
use executor::{current_task, yield_now, AsyncTask, MemType};
use frame_allocator::{ceil_div, frame_alloc_much};
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
    args: Vec<&'a str>,
) -> Result<Arc<dyn AsyncTask>, LinuxError> {
    // copy args, avoid free before pushing.
    let args:Vec<String> = args.into_iter().map(|x|String::from(x)).collect();
    // let mut args = vec![String::from(path)];
    // args.extend(args_v.iter().map(|x| String::from(*x)));
    
    let file = open(path).map_err(from_vfs)?;
    let file_size = file.metadata().unwrap().size;
    let frame_ppn = frame_alloc_much(ceil_div(file_size, PAGE_SIZE));
    let mut buffer = c2rust_buffer(ppn_c(frame_ppn.as_ref().unwrap()[0].0).to_addr() as *mut u8, file_size);
    // let mut buffer = vec![0u8; file.metadata().unwrap().size];
    let rsize = file.read(&mut buffer).map_err(from_vfs)?;

    assert_eq!(rsize, file_size);

    // 读取elf信息
    let elf = xmas_elf::ElfFile::new(&buffer).unwrap();
    let elf_header = elf.header;

    let entry_point = elf.header.pt2.entry_point() as usize;
    assert_eq!(elf_header.pt1.magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
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

    let base = 0;

    // map stack
    user_task.frame_alloc(VirtPage::from_addr(0x7ffff000), MemType::Stack);
    debug!("entry: {:#x}", entry_point);
    user_task.inner_map(|mut inner| {
        inner.heap = heap_bottom as usize;
        inner.entry = entry_point;
        inner.cx.clear();
        inner.cx.set_sp(0x8000_0000); // stack top;
        inner.cx.set_sepc(entry_point);
    });

    // push stack
    let envp = vec![
        "LD_LIBRARY_PATH=/"
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
    auxv.insert(elf::AT_GID, 1000);
    auxv.insert(elf::AT_EGID, 1000);
    auxv.insert(elf::AT_UID, 1000);
    auxv.insert(elf::AT_EUID, 1000);
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
            let virt_addr = ph.virtual_addr() as usize;
            let vpn = virt_addr / PAGE_SIZE;

            let page_count = ceil_div(virt_addr + mem_size, PAGE_SIZE) - vpn;
            let ppn_start = user_task.frame_alloc_much(
                VirtPage::from_addr(virt_addr),
                MemType::CodeSection,
                page_count,
            );

            let page_space = unsafe {
                core::slice::from_raw_parts_mut(
                    (ppn_c(ppn_start).to_addr() + virt_addr % PAGE_SIZE )as _ , file_size)
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
    debug!("sepc: {:#x}", curr_task.inner_map(|x| x.cx.sepc()));
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

pub async fn sys_set_tid_address(tid_ptr: usize) -> Result<usize, LinuxError> {
    debug!("sys_set_tid_address @ tid_ptr: {:#x}", tid_ptr);
    let tid = c2rust_ref(tid_ptr as *mut u32);
    *tid = current_task().get_task_id() as u32;
    Ok(current_task().get_task_id())
}

pub async fn sys_gettid() -> Result<usize, LinuxError> {
    debug!("sys_gettid @ ");
    Ok(current_task().get_task_id())
}
