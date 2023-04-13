use crate::syscall::consts::from_vfs;

use alloc::sync::Arc;
use alloc::vec::Vec;
use executor::{current_task, AsyncTask, UserTask, yield_now, thread};
use fs::mount::open;
use log::debug;

use super::{c2rust_list, c2rust_str, consts::LinuxError};

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

    let task = UserTask::new();

    exec_with_process(task, filename, args).await?;

    Ok(0)
}

pub async fn exec_with_process<'a>(
    task: Arc<dyn AsyncTask>,
    path: &'a str,
    _args: Vec<&'a str>,
) -> Result<Arc<dyn AsyncTask>, LinuxError> {
    let file = open(path).map_err(from_vfs)?;

    let mut buffer = vec![0u8; file.metadata().unwrap().size];
    file.read(&mut buffer).map_err(from_vfs)?;

    // 读取elf信息
    let elf = xmas_elf::ElfFile::new(&buffer).unwrap();
    let elf_header = elf.header;
    let magic = elf_header.pt1.magic;

    let entry_point = elf.header.pt2.entry_point() as usize;
    assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");

    let current_task = current_task();
    info!("current_task: {}", current_task.get_task_id());
    info!("entry_point: {:#x}", entry_point);

    thread::spawn(task.clone());

    yield_now().await;

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

    info!("read file: {}", path);
    Ok(task)
}
