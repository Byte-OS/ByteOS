use executor::current_task;
use fs::{
    mount::{open, rebuild_path},
    pipe::create_pipe,
    OpenFlags, Stat, WaitBlockingRead,
};
use log::debug;

use crate::syscall::{
    c2rust_buffer, c2rust_ref,
    consts::{from_vfs, AT_CWD},
};

use super::{c2rust_str, consts::LinuxError};

pub async fn sys_dup(fd: usize) -> Result<usize, LinuxError> {
    debug!("sys_dup3 @ fd_src: {}", fd);
    let user_task = current_task().as_user_task().unwrap();
    let fd_dst = user_task
        .inner
        .lock()
        .fd_table
        .alloc_fd()
        .ok_or(LinuxError::ENFILE)?;
    sys_dup3(fd, fd_dst).await
}

pub async fn sys_dup3(fd_src: usize, fd_dst: usize) -> Result<usize, LinuxError> {
    debug!("sys_dup3 @ fd_src: {}, fd_dst: {}", fd_src, fd_dst);
    let user_task = current_task().as_user_task().unwrap();
    let mut inner = user_task.inner.lock();
    let file = inner.fd_table.get(fd_src);
    inner.fd_table.set(fd_dst, file);
    Ok(fd_dst)
}

pub async fn sys_read(fd: usize, buf_ptr: usize, count: usize) -> Result<usize, LinuxError> {
    debug!(
        "sys_read @ fd: {} buf_ptr: {:#x} count: {}",
        fd as isize, buf_ptr, count
    );
    let mut buffer = c2rust_buffer(buf_ptr as *mut u8, count);
    let file = current_task()
        .as_user_task()
        .unwrap()
        .inner
        .lock()
        .fd_table
        .get(fd)
        .ok_or(LinuxError::EBADF)?;
    WaitBlockingRead(file, &mut buffer).await.map_err(from_vfs)
}

pub async fn sys_write(fd: usize, buf_ptr: usize, count: usize) -> Result<usize, LinuxError> {
    debug!(
        "sys_write @ fd: {} buf_ptr: {:#x} count: {}",
        fd as isize, buf_ptr, count
    );
    let buffer = c2rust_buffer(buf_ptr as *mut u8, count);
    let user_task = current_task().as_user_task().unwrap();
    let inner = user_task.inner.lock();
    let file = inner.fd_table.get(fd).ok_or(LinuxError::EBADF)?;
    Ok(file.write(buffer).map_err(from_vfs)?)
}

pub async fn sys_close(fd: usize) -> Result<usize, LinuxError> {
    debug!("sys_close @ fd: {}", fd as isize);
    let user_task = current_task().as_user_task().unwrap();
    let mut inner = user_task.inner.lock();
    inner.fd_table.set(fd, None);
    Ok(0)
}

pub async fn sys_mkdir_at(dir_fd: usize, path: usize, mode: usize) -> Result<usize, LinuxError> {
    let path = c2rust_str(path as *mut i8);
    debug!(
        "sys_mkdir_at @ dir_fd: {}, path: {}, mode: {}",
        dir_fd, path, mode
    );
    let user_task = current_task().as_user_task().unwrap();
    let inner = user_task.inner.lock();
    let dir = if dir_fd == AT_CWD {
        open(&inner.curr_dir).map_err(from_vfs)?
    } else {
        inner.fd_table.get(dir_fd).ok_or(LinuxError::EBADF)?
    };
    dir.mkdir(path).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_unlinkat(dir_fd: usize, path: usize, flags: usize) -> Result<usize, LinuxError> {
    let path = c2rust_str(path as *mut i8);
    debug!(
        "sys_unlinkat @ dir_fd: {}, path: {}, flags: {}",
        dir_fd, path, flags
    );
    let user_task = current_task().as_user_task().unwrap();
    let inner = user_task.inner.lock();
    let dir = if dir_fd == AT_CWD {
        open(&inner.curr_dir).map_err(from_vfs)?
    } else {
        inner.fd_table.get(dir_fd).ok_or(LinuxError::EBADF)?
    };
    dir.remove(&rebuild_path(path)).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_openat(
    fd: usize,
    filename: usize,
    flags: usize,
    mode: usize,
) -> Result<usize, LinuxError> {
    let user_task = current_task().as_user_task().unwrap();
    let open_flags = OpenFlags::from_bits_truncate(flags);
    let mut inner = user_task.inner.lock();
    let filename = c2rust_str(filename as *mut i8);
    debug!(
        "sys_openat @ fd: {}, filename: {}, flags: {:?}, mode: {}",
        fd as isize, filename, open_flags, mode
    );
    let path = if fd == AT_CWD {
        inner.curr_dir.clone() + filename
    } else {
        let file = inner.fd_table.get(fd).ok_or(LinuxError::EBADF)?;
        file.path().map_err(from_vfs)? + "/" + filename
    };
    debug!("path: {}", path);
    let file = match open(&path) {
        Ok(file) => Ok(file),
        Err(_) => {
            if open_flags.contains(OpenFlags::O_CREAT) {
                let dir = path.rfind("/").unwrap();
                let dirpath = &path[..dir + 1];
                let filename = &path[dir + 1..];
                Ok(open(dirpath).map_err(from_vfs)?.touch(filename).unwrap())
            } else {
                Err(LinuxError::ENOENT)
            }
        }
    }?;
    debug!("file: {}", file.path().map_err(from_vfs)?);
    let fd = inner.fd_table.alloc_fd().ok_or(LinuxError::ENFILE)?;
    inner.fd_table.set(fd, Some(file));
    debug!("sys_openat @ ret fd: {}", fd);

    Ok(fd)
}

pub async fn sys_fstat(fd: usize, stat_ptr: usize) -> Result<usize, LinuxError> {
    debug!("sys_fstat @ fd: {} stat_ptr: {:#x}", fd, stat_ptr);
    let stat_ref = c2rust_ref(stat_ptr as *mut Stat);
    current_task()
        .as_user_task()
        .unwrap()
        .inner
        .lock()
        .fd_table
        .get(fd)
        .ok_or(LinuxError::EBADF)?
        .stat(stat_ref)
        .map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_pipe2(fds_ptr: usize, _unknown: usize) -> Result<usize, LinuxError> {
    debug!(
        "sys_pipe2 @ fds_ptr: {:#x}, _unknown: {}",
        fds_ptr, _unknown
    );
    let fds = c2rust_buffer(fds_ptr as *mut u32, 2);
    let user_task = current_task().as_user_task().unwrap();
    let mut inner = user_task.inner.lock();

    let (rx, tx) = create_pipe();

    let rx_fd = inner.fd_table.alloc_fd().ok_or(LinuxError::ENFILE)?;
    inner.fd_table.set(rx_fd, Some(rx));
    fds[0] = rx_fd as u32;

    let tx_fd = inner.fd_table.alloc_fd().ok_or(LinuxError::ENFILE)?;
    inner.fd_table.set(tx_fd, Some(tx));
    fds[1] = tx_fd as u32;

    debug!("sys_pipe2 ret: {} {}", rx_fd as u32, tx_fd as u32);
    Ok(0)
}
