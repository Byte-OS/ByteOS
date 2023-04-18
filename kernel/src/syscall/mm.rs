use arch::{VirtAddr, VirtPage, PAGE_SIZE};
use executor::current_task;
use log::debug;

use crate::syscall::{
    c2rust_buffer,
    consts::{from_vfs, MapFlags},
};

use super::consts::LinuxError;

pub async fn sys_brk(addr: isize) -> Result<usize, LinuxError> {
    debug!("sys_brk @ increment: {}", addr);
    let user_task = current_task().as_user_task().unwrap();
    if addr == 0 {
        Ok(user_task.heap())
    } else {
        debug!("alloc pos: {}", addr - user_task.heap() as isize);
        Ok(user_task.sbrk(addr - user_task.heap() as isize))
    }
}

pub async fn sys_mmap(
    start: usize,
    len: usize,
    prot: usize,
    flags: usize,
    fd: usize,
    off: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_mmap @ start: {:#x}, len: {:#x}, prot: {}, flags: {}, fd: {}, offset: {}",
        start, len, prot, flags, fd, off
    );
    let flags = MapFlags::from_bits_truncate(flags as _);
    let user_task = current_task().as_user_task().unwrap();
    let file = user_task.inner_map(|x| x.fd_table.get(fd).clone());

    let addr = if start == 0 {
        user_task.get_last_free_addr()
    } else {
        VirtAddr::new(start)
    };

    debug!("sys_mmap @ free addr: {}", addr);

    if flags.contains(MapFlags::MAP_SHARED) {
        user_task.frame_alloc_much(
            VirtPage::from_addr(addr.into()),
            executor::MemType::Shared(file.clone(), addr.into(), len),
            (len + PAGE_SIZE - 1) / PAGE_SIZE,
        );

        let mut buffer = c2rust_buffer(usize::from(addr) as *mut u8, len);

        if let Some(file) = file {
            let offset = file.seek(fs::SeekFrom::CURRENT(0)).map_err(from_vfs)?;
            file.seek(fs::SeekFrom::SET(0)).map_err(from_vfs)?;
            let len = file.read(&mut buffer).map_err(from_vfs)?;
            file.seek(fs::SeekFrom::SET(offset)).map_err(from_vfs)?;
            debug!("read len: {}", len);
        }
    }
    Ok(addr.into())
}

pub async fn sys_munmap(start: usize, len: usize) -> Result<usize, LinuxError> {
    debug!("sys_munmap @ start: {:#x}, len: {}", start, len);
    let current_task = current_task().as_user_task().unwrap();

    current_task.inner_map(|mut x| {
        x.memset
            .drain_filter(|x| (start..start + len).contains(&x.vpn.to_addr()));
    });
    Ok(0)
}
