use alloc::vec::Vec;
use executor::current_task;
use fs::mount::{open, umount};
use fs::pipe::create_pipe;
use fs::{OpenFlags, SeekFrom, Stat, StatFS, TimeSpec, WaitBlockingRead, UTIME_NOW};
use log::debug;

use crate::syscall::consts::{fcntl_cmd, from_vfs, IoVec, AT_CWD};
use crate::syscall::func::{c2rust_buffer, c2rust_ref, c2rust_str, timespc_now};

use super::consts::LinuxError;

pub async fn sys_dup(fd: usize) -> Result<usize, LinuxError> {
    debug!("sys_dup3 @ fd_src: {}", fd);
    let user_task = current_task().as_user_task().unwrap();
    let fd_dst = user_task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    sys_dup3(fd, fd_dst).await
}

pub async fn sys_dup3(fd_src: usize, fd_dst: usize) -> Result<usize, LinuxError> {
    debug!("sys_dup3 @ fd_src: {}, fd_dst: {}", fd_src, fd_dst);
    let user_task = current_task().as_user_task().unwrap();
    let file = user_task.get_fd(fd_src);
    user_task.set_fd(fd_dst, file);
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
        .get_fd(fd)
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
    let file = user_task.get_fd(fd).ok_or(LinuxError::EBADF)?;
    Ok(file.write(buffer).map_err(from_vfs)?)
}

pub async fn sys_readv(fd: usize, iov: usize, iocnt: usize) -> Result<usize, LinuxError> {
    debug!("sys_readv @ fd: {}, iov: {:#x}, iocnt: {}", fd, iov, iocnt);

    let mut rsize = 0;

    let iov = c2rust_buffer(iov as *mut IoVec, iocnt);
    let user_task = current_task().as_user_task().unwrap();
    let file = user_task.get_fd(fd).ok_or(LinuxError::EBADF)?;

    for io in iov {
        let buffer = c2rust_buffer(io.base as *mut u8, io.len);
        rsize += file.read(buffer).map_err(from_vfs)?;
    }

    Ok(rsize)
}

pub async fn sys_writev(fd: usize, iov: usize, iocnt: usize) -> Result<usize, LinuxError> {
    debug!("sys_writev @ fd: {}, iov: {:#x}, iocnt: {}", fd, iov, iocnt);
    let mut wsize = 0;

    let iov = c2rust_buffer(iov as *mut IoVec, iocnt);
    let user_task = current_task().as_user_task().unwrap();
    let file = user_task.get_fd(fd).ok_or(LinuxError::EBADF)?;

    for io in iov {
        let buffer = c2rust_buffer(io.base as *mut u8, io.len);
        wsize += file.write(buffer).map_err(from_vfs)?;
    }

    Ok(wsize)
}

pub async fn sys_close(fd: usize) -> Result<usize, LinuxError> {
    debug!("sys_close @ fd: {}", fd as isize);
    let user_task = current_task().as_user_task().unwrap();
    user_task.set_fd(fd, None);
    Ok(0)
}

pub async fn sys_mkdir_at(dir_fd: usize, path: usize, mode: usize) -> Result<usize, LinuxError> {
    let path = c2rust_str(path as *mut i8);
    debug!(
        "sys_mkdir_at @ dir_fd: {}, path: {}, mode: {}",
        dir_fd, path, mode
    );
    let user_task = current_task().as_user_task().unwrap();
    let dir = if dir_fd == AT_CWD {
        open(&user_task.inner.lock().curr_dir).map_err(from_vfs)?
    } else {
        user_task.get_fd(dir_fd).ok_or(LinuxError::EBADF)?
    };
    dir.mkdir(path).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_unlinkat(dir_fd: usize, path: usize, flags: usize) -> Result<usize, LinuxError> {
    let path = c2rust_str(path as *mut i8);
    debug!(
        "sys_unlinkat @ dir_fd: {}, path: {}, flags: {}",
        dir_fd as isize, path, flags
    );
    let user_task = current_task().as_user_task().unwrap();
    let dir = if dir_fd == AT_CWD {
        open(&user_task.inner.lock().curr_dir).map_err(from_vfs)?
    } else {
        user_task.get_fd(dir_fd).ok_or(LinuxError::EBADF)?
    };
    let full_path = format!("{}/{}", dir.path().map_err(from_vfs)?, path);
    let mut paths = full_path.split("/").fold(Vec::new(), |mut p, x| match x {
        "." | "" => p,
        ".." => {
            p.pop();
            p
        }
        _ => {
            p.push(x);
            p
        }
    });
    match paths.pop() {
        Some(filename) => {
            let dir_path = format!("/{}", paths.join("/"));
            open(&dir_path)
                .map_err(from_vfs)?
                .remove(filename)
                .map_err(from_vfs)?;
            Ok(0)
        }
        None => Err(LinuxError::EINVAL),
    }
}

pub async fn sys_openat(
    fd: usize,
    filename: usize,
    flags: usize,
    mode: usize,
) -> Result<usize, LinuxError> {
    let user_task = current_task().as_user_task().unwrap();
    let open_flags = OpenFlags::from_bits_truncate(flags);
    let filename = c2rust_str(filename as *mut i8);
    debug!(
        "sys_openat @ fd: {}, filename: {}, flags: {:?}, mode: {}",
        fd as isize, filename, open_flags, mode
    );
    let path = if fd == AT_CWD {
        user_task.inner.lock().curr_dir.clone() + filename
    } else {
        let file = user_task.get_fd(fd).ok_or(LinuxError::EBADF)?;
        file.path().map_err(from_vfs)? + "/" + filename
    };
    let file = match open(&path) {
        Ok(file) => Ok(file),
        Err(_) => {
            if open_flags.contains(OpenFlags::O_CREAT) {
                let dir = path.rfind("/").unwrap();
                let dirpath = &path[..dir + 1];
                let filename = &path[dir + 1..];
                let f = open(dirpath).map_err(from_vfs)?.touch(filename).unwrap();
                debug!("f: {:?}", f.metadata());
                Ok(f)
            } else {
                Err(LinuxError::ENOENT)
            }
        }
    }?;
    debug!("file: {:?}", file.path());
    let fd = user_task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    user_task.set_fd(fd, Some(file));
    debug!("sys_openat @ ret fd: {}", fd);

    Ok(fd)
}

pub async fn sys_fstat(fd: usize, stat_ptr: usize) -> Result<usize, LinuxError> {
    debug!("sys_fstat @ fd: {} stat_ptr: {:#x}", fd, stat_ptr);
    let stat_ref = c2rust_ref(stat_ptr as *mut Stat);
    current_task()
        .as_user_task()
        .unwrap()
        .get_fd(fd)
        .ok_or(LinuxError::EBADF)?
        .stat(stat_ref)
        .map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_fstatat(
    dir_fd: usize,
    filename_ptr: usize,
    stat_ptr: usize,
) -> Result<usize, LinuxError> {
    let filename = c2rust_str(filename_ptr as *mut i8);
    debug!(
        "sys_fstatat @ dir_fd: {}, filename:{}, stat_ptr: {:#x}",
        dir_fd, filename, stat_ptr
    );
    let stat = c2rust_ref(stat_ptr as *mut Stat);

    let user_task = current_task().as_user_task().unwrap();

    let dir = if dir_fd == AT_CWD {
        open(&user_task.inner.lock().curr_dir).map_err(from_vfs)
    } else {
        user_task.get_fd(dir_fd).ok_or(LinuxError::EBADF)
    }?;

    open(&format!("{}/{}", dir.path().map_err(from_vfs)?, filename))
        .map_err(from_vfs)?
        .stat(stat)
        .map_err(from_vfs)?;

    Ok(0)
}

pub async fn sys_statfs(filename_ptr: usize, statfs_ptr: usize) -> Result<usize, LinuxError> {
    debug!(
        "sys_statfs @ filename_ptr: {:#x}, statfs_ptr: {:#x}",
        filename_ptr, statfs_ptr
    );
    let path = c2rust_str(filename_ptr as *mut i8);
    let statfs = c2rust_ref(statfs_ptr as *mut StatFS);
    open(path)
        .map_err(from_vfs)?
        .statfs(statfs)
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

    let (rx, tx) = create_pipe();

    let rx_fd = user_task.alloc_fd().ok_or(LinuxError::ENFILE)?;
    user_task.set_fd(rx_fd, Some(rx));
    fds[0] = rx_fd as u32;

    let tx_fd = user_task.alloc_fd().ok_or(LinuxError::ENFILE)?;
    user_task.set_fd(tx_fd, Some(tx));
    fds[1] = tx_fd as u32;

    debug!("sys_pipe2 ret: {} {}", rx_fd as u32, tx_fd as u32);
    Ok(0)
}

pub async fn sys_pread(
    fd: usize,
    ptr: usize,
    len: usize,
    offset: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_pread @ fd: {}, ptr: {:#x}, len: {}, offset: {}",
        fd, ptr, len, offset
    );
    let buffer = c2rust_buffer(ptr as *mut u8, len);

    let file = current_task()
        .as_user_task()
        .unwrap()
        .get_fd(fd)
        .ok_or(LinuxError::EBADF)?;

    let old_off = file.seek(SeekFrom::CURRENT(0)).map_err(from_vfs)?;

    file.seek(SeekFrom::SET(offset)).map_err(from_vfs)?;

    let result = file.read(buffer).map_err(from_vfs);
    file.seek(SeekFrom::SET(old_off)).map_err(from_vfs)?;
    result
}

pub async fn sys_mount(
    special: usize,
    dir: usize,
    fstype: usize,
    flags: usize,
    data: usize,
) -> Result<usize, LinuxError> {
    let special = c2rust_str(special as *mut i8);
    let dir = c2rust_str(dir as *mut i8);
    let fstype = c2rust_str(fstype as *mut i8);

    debug!(
        "sys_mount @ special: {}, dir: {}, fstype: {}, flags: {}, data: {:#x}",
        special, dir, fstype, flags, data
    );

    let file = open(special).map_err(from_vfs)?;
    file.mount(dir).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_umount2(special: usize, flags: usize) -> Result<usize, LinuxError> {
    let special = c2rust_str(special as *mut i8);
    debug!("sys_umount @ special: {}, flags: {}", special, flags);
    match special.starts_with("/dev") {
        true => {
            let dev = open(special).map_err(from_vfs)?;
            dev.umount().map_err(from_vfs)?;
        }
        false => {
            umount(special).map_err(from_vfs)?;
        }
    };

    Ok(0)
}

pub async fn sys_getdents64(fd: usize, buf_ptr: usize, len: usize) -> Result<usize, LinuxError> {
    debug!(
        "sys_getdents64 @ fd: {}, buf_ptr: {:#X}, len: {}",
        fd, buf_ptr, len
    );

    let file = current_task().as_user_task().unwrap().get_fd(fd).unwrap();

    let buffer = c2rust_buffer(buf_ptr as *mut u8, len);
    file.getdents(buffer).map_err(from_vfs)
}

pub fn sys_lseek(fd: usize, offset: usize, whence: usize) -> Result<usize, LinuxError> {
    debug!(
        "sys_lseek @ fd {}, offset: {}, whench: {}",
        fd, offset as isize, whence
    );

    let usre_task = current_task().as_user_task().unwrap();
    let file = usre_task.get_fd(fd).ok_or(LinuxError::EBADF)?;
    let seek_from = match whence {
        0 => SeekFrom::SET(offset),
        1 => SeekFrom::CURRENT(offset as isize),
        2 => SeekFrom::END(offset as isize),
        _ => return Err(LinuxError::EINVAL),
    };
    file.seek(seek_from).map_err(from_vfs)
}

pub async fn sys_ioctl(
    fd: usize,
    request: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "ioctl: fd: {}, request: {:#x}, args: {:#x} {:#x} {:#x}",
        fd, request, arg1, arg2, arg3
    );

    Ok(0)
}

pub async fn sys_fcntl(fd: usize, cmd: usize, arg: usize) -> Result<usize, LinuxError> {
    debug!("fcntl: fd: {}, cmd: {:#x}, arg: {}", fd, cmd, arg);

    match cmd {
        fcntl_cmd::DUPFD_CLOEXEC => sys_dup(fd).await,
        _ => Err(LinuxError::EPERM),
    }
}

/// information source: https://man7.org/linux/man-pages/man2/utimensat.2.html
///
/// Updated file timestamps are set to the greatest value supported
/// by the filesystem that is not greater than the specified time.
///
/// If the tv_nsec field of one of the timespec structures has the
/// special value UTIME_NOW, then the corresponding file timestamp is
/// set to the current time.  If the tv_nsec field of one of the
/// timespec structures has the special value UTIME_OMIT, then the
/// corresponding file timestamp is left unchanged.  In both of these
/// cases, the value of the corresponding tv_sec field is ignored.
///
/// If times is NULL, then both timestamps are set to the current
/// time.
pub async fn sys_utimensat(
    dir_fd: usize,
    path: usize,
    times_ptr: usize,
    flags: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_utimensat @ dir_fd: {}, path: {:#x}, times_ptr: {:#x}, flags: {}",
        dir_fd, path, times_ptr, flags
    );
    // build times
    let mut times = match times_ptr == 0 {
        true => {
            vec![timespc_now(), timespc_now()]
        }
        false => {
            let ts = c2rust_buffer(times_ptr as *mut TimeSpec, 2);
            let mut times = vec![];
            for i in 0..2 {
                if ts[i].nsec == UTIME_NOW {
                    times.push(timespc_now());
                } else {
                    times.push(ts[i]);
                }
            }
            times
        }
    };

    let user_task = current_task().as_user_task().unwrap();

    let dir = if dir_fd == AT_CWD {
        open(&user_task.inner.lock().curr_dir).map_err(from_vfs)
    } else {
        user_task.get_fd(dir_fd).ok_or(LinuxError::EBADF)
    }?;

    let file = if path == 0 {
        dir
    } else {
        let path = c2rust_str(path as *const i8);
        let file_path = format!("{}/{}", dir.path().map_err(from_vfs)?, path);
        open(&file_path).map_err(from_vfs)?
    };

    file.utimes(&mut times).map_err(from_vfs)?;
    Ok(0)
}
