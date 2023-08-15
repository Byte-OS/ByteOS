use alloc::string::{String, ToString};
use vfscore::FileType;
use core::cmp;
use num_traits::FromPrimitive;

use alloc::sync::Arc;
use alloc::vec::Vec;
use arch::VirtAddr;
use bit_field::BitArray;
use executor::{
    current_task, current_user_task, yield_now, AsyncTask, FileItem, FileItemInterface, FileOptions,
};
use fs::mount::{open, rebuild_path, umount};
use fs::pipe::create_pipe;
use fs::{
    INodeInterface, OpenFlags, PollEvent, PollFd, SeekFrom, Stat, StatFS, StatMode, TimeSpec,
    UTIME_NOW,
};
use log::{debug, warn};

use crate::epoll::{EpollEvent, EpollFile};
use crate::socket::Socket;
use crate::syscall::consts::{from_vfs, FcntlCmd, IoVec, AT_CWD};
use crate::syscall::func::timespc_now;
use crate::syscall::time::current_nsec;

use super::consts::{LinuxError, UserRef};

pub async fn sys_dup(fd: usize) -> Result<usize, LinuxError> {
    debug!("sys_dup3 @ fd_src: {}", fd);
    let user_task = current_user_task();
    let fd_dst = user_task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    sys_dup3(fd, fd_dst).await
}

pub async fn sys_dup3(fd_src: usize, fd_dst: usize) -> Result<usize, LinuxError> {
    debug!("sys_dup3 @ fd_src: {}, fd_dst: {}", fd_src, fd_dst);
    let user_task = current_user_task();
    let file = user_task.get_fd(fd_src);
    user_task.set_fd(fd_dst, file);
    Ok(fd_dst)
}

pub async fn sys_read(fd: usize, buf_ptr: UserRef<u8>, count: usize) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_read @ fd: {} buf_ptr: {:?} count: {}",
        task.get_task_id(),
        fd as isize,
        buf_ptr,
        count
    );

    let buffer = buf_ptr.slice_mut_with_len(count);
    let file = task.get_fd(fd).ok_or(LinuxError::EBADF)?;
    file.async_read(buffer).await.map_err(from_vfs)
}

pub async fn sys_write(fd: usize, buf_ptr: VirtAddr, count: usize) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_write @ fd: {} buf_ptr: {:?} count: {}",
        task.get_task_id(),
        fd as isize,
        buf_ptr,
        count
    );

    let buffer = buf_ptr.slice_with_len(count);
    let file = current_user_task().get_fd(fd).ok_or(LinuxError::EBADF)?;
    if let Ok(_) = file.get_bare_file().downcast_arc::<Socket>() {
        yield_now().await;
    }
    file.async_write(buffer).await.map_err(from_vfs)
}

pub async fn sys_readv(fd: usize, iov: UserRef<IoVec>, iocnt: usize) -> Result<usize, LinuxError> {
    debug!("sys_readv @ fd: {}, iov: {}, iocnt: {}", fd, iov, iocnt);

    let mut rsize = 0;

    let iov = iov.slice_mut_with_len(iocnt);
    let user_task = current_task().as_user_task().unwrap();
    let file = user_task.get_fd(fd).ok_or(LinuxError::EBADF)?;

    for io in iov {
        let buffer = UserRef::<u8>::from(io.base).slice_mut_with_len(io.len);
        rsize += file.read(buffer).map_err(from_vfs)?;
    }

    Ok(rsize)
}

pub async fn sys_writev(fd: usize, iov: UserRef<IoVec>, iocnt: usize) -> Result<usize, LinuxError> {
    debug!("sys_writev @ fd: {}, iov: {}, iocnt: {}", fd, iov, iocnt);
    let mut wsize = 0;

    let iov = iov.slice_mut_with_len(iocnt);
    let user_task = current_task().as_user_task().unwrap();
    let file = user_task.get_fd(fd).ok_or(LinuxError::EBADF)?;

    for io in iov {
        let buffer = UserRef::<u8>::from(io.base).slice_mut_with_len(io.len);
        wsize += file.write(buffer).map_err(from_vfs)?;
    }

    Ok(wsize)
}

pub async fn sys_close(fd: usize) -> Result<usize, LinuxError> {
    let user_task = current_user_task();
    debug!(
        "[task {}] sys_close @ fd: {}",
        user_task.get_task_id(),
        fd as isize
    );
    user_task.set_fd(fd, None);
    Ok(0)
}

pub async fn sys_mkdir_at(
    dir_fd: usize,
    path: UserRef<i8>,
    mode: usize,
) -> Result<usize, LinuxError> {
    let path = path.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    debug!(
        "sys_mkdir_at @ dir_fd: {}, path: {}, mode: {}",
        dir_fd as isize, path, mode
    );
    let user_task = current_task().as_user_task().unwrap();
    let dir = if dir_fd == AT_CWD {
        if path.starts_with("/") {
            FileItem::fs_open("/", Default::default())
        } else {
            FileItem::fs_open(&user_task.pcb.lock().curr_dir, Default::default())
        }
        .map_err(from_vfs)?
    } else {
        user_task.get_fd(dir_fd).ok_or(LinuxError::EBADF)?
    };
    if path == "/" {
        return Err(LinuxError::EEXIST);
    }

    let path_str = rebuild_path(path);
    let paths: Vec<&str> = path_str.split("/").collect();
    let mut pfile = dir.inner;
    for i in paths.into_iter().filter(|x| *x != "") {
        let f = pfile.open(i, OpenFlags::O_RDWR);
        if f.is_err() {
            pfile.mkdir(i).map_err(from_vfs)?;
        } else {
            pfile = f.unwrap();
        }
    }
    // a simple method
    // dir.mkdir(path).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_unlinkat(
    dir_fd: usize,
    path: UserRef<i8>,
    flags: usize,
) -> Result<usize, LinuxError> {
    let path = path.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    debug!(
        "sys_unlinkat @ dir_fd: {}, path: {}, flags: {}",
        dir_fd as isize, path, flags
    );
    let user_task = current_task().as_user_task().unwrap();
    let dir = if dir_fd == AT_CWD {
        if path.starts_with("/") {
            FileItem::fs_open("/", Default::default())
        } else {
            FileItem::fs_open(&user_task.pcb.lock().curr_dir, Default::default())
        }
        .map_err(from_vfs)?
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
    filename: UserRef<i8>,
    flags: usize,
    mode: usize,
) -> Result<usize, LinuxError> {
    let user_task = current_task().as_user_task().unwrap();
    let open_flags = OpenFlags::from_bits_truncate(flags);
    let filename = if filename.is_valid() {
        filename.get_cstr().map_err(|_| LinuxError::EINVAL)?
    } else {
        ""
    };
    debug!(
        "sys_openat @ fd: {}, filename: {}, flags: {:?}, mode: {}",
        fd as isize, filename, open_flags, mode
    );
    let mut options = FileOptions::R | FileOptions::X;
    if open_flags.contains(OpenFlags::O_WRONLY)
        || open_flags.contains(OpenFlags::O_RDWR)
        || open_flags.contains(OpenFlags::O_ACCMODE)
    {
        options = options.union(FileOptions::W);
    }
    let path = if filename.starts_with("/") {
        String::from(filename)
    } else {
        if fd == AT_CWD {
            user_task.pcb.lock().curr_dir.clone() + "/" + filename
        } else {
            let file = user_task.get_fd(fd).ok_or(LinuxError::EBADF)?;
            file.path().map_err(from_vfs)?.to_string() + "/" + filename
        }
    };
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
    if open_flags.contains(OpenFlags::O_APPEND) {
        file.seek(SeekFrom::END(0))
            .expect("can't seek to end of file");
    }
    let fd = user_task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    user_task.set_fd(fd, Some(FileItem::new(file, options)));
    debug!("sys_openat @ ret fd: {}", fd);
    Ok(fd)
}

pub async fn sys_faccess_at(
    fd: usize,
    filename: UserRef<i8>,
    mode: usize,
    flags: usize,
) -> Result<usize, LinuxError> {
    let user_task = current_task().as_user_task().unwrap();
    let open_flags = OpenFlags::from_bits_truncate(flags);
    let filename = if filename.is_valid() {
        filename.get_cstr().map_err(|_| LinuxError::EINVAL)?
    } else {
        ""
    };
    debug!(
        "sys_accessat @ fd: {}, filename: {}, flags: {:?}, mode: {}",
        fd as isize, filename, open_flags, mode
    );
    let path = if filename.starts_with("/") {
        String::from(filename)
    } else {
        if fd == AT_CWD {
            user_task.pcb.lock().curr_dir.clone() + "/" + filename
        } else {
            let file = user_task.get_fd(fd).ok_or(LinuxError::EBADF)?;
            file.path().map_err(from_vfs)?.to_string() + "/" + filename
        }
    };
    open(&path).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_fstat(fd: usize, stat_ptr: UserRef<Stat>) -> Result<usize, LinuxError> {
    debug!("sys_fstat @ fd: {} stat_ptr: {}", fd, stat_ptr);
    let stat_ref = stat_ptr.get_mut();
    current_task()
        .as_user_task()
        .unwrap()
        .get_fd(fd)
        .ok_or(LinuxError::EBADF)?
        .stat(stat_ref)
        .map_err(from_vfs)?;
    stat_ref.mode |= StatMode::OWNER_MASK | StatMode::GROUP_MASK | StatMode::OTHER_MASK;
    Ok(0)
}

pub async fn sys_fstatat(
    dir_fd: usize,
    filename_ptr: UserRef<i8>,
    stat_ptr: UserRef<Stat>,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_fstatat @ dir_fd: {}, filename:{}, stat_ptr: {}",
        dir_fd as isize, filename_ptr, stat_ptr
    );
    let filename = filename_ptr.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    debug!(
        "sys_fstatat @ dir_fd: {}, filename:{}, stat_ptr: {}",
        dir_fd as isize, filename, stat_ptr
    );
    let stat = stat_ptr.get_mut();

    let user_task = current_task().as_user_task().unwrap();

    let path = if filename.starts_with("/") {
        String::from(filename)
    } else {
        if dir_fd == AT_CWD {
            user_task.pcb.lock().curr_dir.clone() + "/" + filename
        } else {
            let file = user_task.get_fd(dir_fd).ok_or(LinuxError::EBADF)?;
            file.path().map_err(from_vfs)?.to_string() + "/" + filename
        }
    };

    open(&path)
        .map_err(from_vfs)?
        .stat(stat)
        .map_err(from_vfs)?;

    stat.mode |= StatMode::OWNER_MASK | StatMode::GROUP_MASK | StatMode::OTHER_MASK;
    Ok(0)
}

pub async fn sys_statfs(
    filename_ptr: UserRef<i8>,
    statfs_ptr: UserRef<StatFS>,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_statfs @ filename_ptr: {}, statfs_ptr: {}",
        filename_ptr, statfs_ptr
    );
    let path = filename_ptr.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    let statfs = statfs_ptr.get_mut();
    open(path)
        .map_err(from_vfs)?
        .statfs(statfs)
        .map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_pipe2(fds_ptr: UserRef<u32>, _unknown: usize) -> Result<usize, LinuxError> {
    debug!("sys_pipe2 @ fds_ptr: {}, _unknown: {}", fds_ptr, _unknown);
    let fds = fds_ptr.slice_mut_with_len(2);
    let user_task = current_task().as_user_task().unwrap();

    let (rx, tx) = create_pipe();

    let rx_fd = user_task.alloc_fd().ok_or(LinuxError::ENFILE)?;
    user_task.set_fd(rx_fd, Some(FileItem::new(rx, Default::default())));
    fds[0] = rx_fd as u32;

    let tx_fd = user_task.alloc_fd().ok_or(LinuxError::ENFILE)?;
    user_task.set_fd(tx_fd, Some(FileItem::new(tx, Default::default())));
    fds[1] = tx_fd as u32;

    debug!("sys_pipe2 ret: {} {}", rx_fd as u32, tx_fd as u32);
    Ok(0)
}

pub async fn sys_pread(
    fd: usize,
    ptr: UserRef<u8>,
    len: usize,
    offset: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_pread @ fd: {}, ptr: {}, len: {}, offset: {}",
        fd, ptr, len, offset
    );
    let buffer = ptr.slice_mut_with_len(len);

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

pub async fn sys_pwrite(
    fd: usize,
    buf_ptr: VirtAddr,
    count: usize,
    offset: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_write @ fd: {} buf_ptr: {:?} count: {}",
        fd as isize, buf_ptr, count
    );
    let buffer = buf_ptr.slice_with_len(count);
    let file = current_user_task().get_fd(fd).ok_or(LinuxError::EBADF)?;
    // file.async_write(buffer).await.map_err(from_vfs)
    let old_off = file.seek(SeekFrom::CURRENT(0)).map_err(from_vfs)?;

    file.seek(SeekFrom::SET(offset)).map_err(from_vfs)?;

    let result: Result<usize, LinuxError> = file.write(buffer).map_err(from_vfs);
    file.seek(SeekFrom::SET(old_off)).map_err(from_vfs)?;
    result
}

pub async fn sys_mount(
    special: UserRef<i8>,
    dir: UserRef<i8>,
    fstype: UserRef<i8>,
    flags: usize,
    data: usize,
) -> Result<usize, LinuxError> {
    let special = special.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    let dir = dir.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    let fstype = fstype.get_cstr().map_err(|_| LinuxError::EINVAL)?;

    debug!(
        "sys_mount @ special: {}, dir: {}, fstype: {}, flags: {}, data: {:#x}",
        special, dir, fstype, flags, data
    );

    let file = open(special).map_err(from_vfs)?;
    file.mount(dir).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_umount2(special: UserRef<i8>, flags: usize) -> Result<usize, LinuxError> {
    let special = special.get_cstr().map_err(|_| LinuxError::EINVAL)?;
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

pub async fn sys_getdents64(
    fd: usize,
    buf_ptr: UserRef<u8>,
    len: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_getdents64 @ fd: {}, buf_ptr: {}, len: {}",
        fd, buf_ptr, len
    );

    let file = current_task().as_user_task().unwrap().get_fd(fd).unwrap();

    let buffer = buf_ptr.slice_mut_with_len(len);
    let res = file.getdents(buffer).map_err(from_vfs);
    res
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
    let file = current_user_task().get_fd(fd).ok_or(LinuxError::EINVAL)?;
    file.ioctl(request, arg1).map_err(from_vfs)
    // Ok(0)
}

pub async fn sys_fcntl(fd: usize, cmd: usize, arg: usize) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] fcntl: fd: {}, cmd: {:#x}, arg: {}",
        task.get_task_id(),
        fd,
        cmd,
        arg
    );
    let cmd = FromPrimitive::from_usize(cmd).ok_or(LinuxError::EINVAL)?;
    let mut file = task.get_fd(fd).ok_or(LinuxError::EBADF)?;

    match cmd {
        FcntlCmd::DUPFD | FcntlCmd::DUPFDCLOEXEC => sys_dup(fd).await,
        FcntlCmd::GETFD => Ok(1),
        FcntlCmd::GETFL => Ok(file.flags.bits()),
        FcntlCmd::SETFL => {
            file.flags = OpenFlags::from_bits_truncate(arg);
            task.set_fd(fd, Some(file));
            Ok(0)
        }
        _ => Ok(0),
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
    path: UserRef<u8>,
    times_ptr: UserRef<TimeSpec>,
    flags: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_utimensat @ dir_fd: {}, path: {}, times_ptr: {}, flags: {}",
        dir_fd, path, times_ptr, flags
    );
    // build times
    let mut times = match !times_ptr.is_valid() {
        true => {
            vec![timespc_now(), timespc_now()]
        }
        false => {
            let ts = times_ptr.slice_mut_with_len(2);
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
        FileItem::fs_open(&user_task.pcb.lock().curr_dir, Default::default()).map_err(from_vfs)
    } else {
        user_task.get_fd(dir_fd).ok_or(LinuxError::EBADF)
    }?;

    let file = if !path.is_valid() {
        dir
    } else {
        let path = path.get_cstr().map_err(|_| LinuxError::EINVAL)?;
        let file_path = if path.starts_with("/") {
            String::from(path)
        } else {
            format!("{}/{}", dir.path().map_err(from_vfs)?, path)
        };
        FileItem::fs_open(&file_path, Default::default()).map_err(from_vfs)?
    };

    file.utimes(&mut times).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_readlinkat(
    dir_fd: usize,
    path: UserRef<i8>,
    buffer: UserRef<u8>,
    buffer_size: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_readlinkat @ dir_fd: {}, path: {}, buffer: {}, size: {}",
        dir_fd, path, buffer, buffer_size
    );
    let filename = path.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    let buffer = buffer.slice_mut_with_len(buffer_size);
    debug!("readlinkat @ filename: {}", filename);
    let user_task = current_task().as_user_task().unwrap();

    let dir = if dir_fd == AT_CWD {
        FileItem::fs_open(&user_task.pcb.lock().curr_dir, Default::default()).map_err(from_vfs)
    } else {
        user_task.get_fd(dir_fd).ok_or(LinuxError::EBADF)
    }?;

    let ftype = dir.open(filename, OpenFlags::NONE)
        .map_err(from_vfs)?
        .metadata()
        .map_err(from_vfs)?
        .file_type;

    if FileType::Link != ftype {
        return Err(LinuxError::EINVAL);
    }

    let file_path = open(&format!("{}/{}", dir.path().map_err(from_vfs)?, filename))
        .map_err(from_vfs)?
        .resolve_link()
        .map_err(from_vfs)?;

    let bytes = file_path.as_bytes();

    let rlen = cmp::min(bytes.len(), buffer_size);

    buffer[..rlen].copy_from_slice(&bytes[..rlen]);
    debug!("sys_readlinkat: rlen: {}", rlen);
    Ok(rlen)
}

pub async fn sys_sendfile(
    out_fd: usize,
    in_fd: usize,
    offset: usize,
    count: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "out_fd: {}  in_fd: {}  offset: {:#x}   count: {}",
        out_fd, in_fd, offset, count
    );

    if offset != 0 {
        warn!("sys_sendfile offset neq 0");
    }

    let task = current_user_task();

    let out_file = task.get_fd(out_fd).ok_or(LinuxError::EINVAL)?;
    let in_file = task.get_fd(in_fd).ok_or(LinuxError::EINVAL)?;

    let rlen = cmp::min(
        in_file.metadata().map_err(from_vfs)?.size
            - in_file.seek(SeekFrom::CURRENT(0)).map_err(from_vfs)?,
        count,
    );

    let mut buffer = vec![0u8; rlen];

    in_file.read(&mut buffer).map_err(from_vfs)?;
    out_file.write(&buffer).map_err(from_vfs)
}

/// TODO: improve it.
pub async fn sys_ppoll(
    poll_fds_ptr: UserRef<PollFd>,
    nfds: usize,
    timeout_ptr: UserRef<TimeSpec>,
    sigmask_ptr: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_ppoll @ poll_fds_ptr: {}, nfds: {}, timeout_ptr: {}, sigmask_ptr: {:#X}",
        poll_fds_ptr, nfds, timeout_ptr, sigmask_ptr
    );
    let task = current_user_task();
    let poll_fds = poll_fds_ptr.slice_mut_with_len(nfds);
    let etime = if timeout_ptr.is_valid() {
        current_nsec() + timeout_ptr.get_ref().to_nsec()
    } else {
        usize::MAX
    };
    let n = loop {
        let mut num = 0;
        for i in 0..nfds {
            poll_fds[i].revents = task
                .get_fd(poll_fds[i].fd as _)
                .map_or(PollEvent::NONE, |x| {
                    x.poll(poll_fds[i].events.clone()).unwrap()
                });
            if poll_fds[i].revents != PollEvent::NONE {
                num += 1;
            }
        }

        if current_nsec() >= etime || num > 0 {
            break num;
        }
        yield_now().await;
    };
    Ok(n)
}

/// TODO: improve it.
pub async fn sys_pselect(
    mut max_fdp1: usize,
    readfds: UserRef<usize>,
    writefds: UserRef<usize>,
    exceptfds: UserRef<usize>,
    timeout_ptr: UserRef<TimeSpec>,
    sigmask: usize,
) -> Result<usize, LinuxError> {
    let user_task = current_user_task();
    debug!(
        "[task {}] sys_pselect @ max_fdp1: {}, readfds: {}, writefds: {}, exceptfds: {}, tsptr: {}, sigmask: {:#X}",
        user_task.get_task_id(), max_fdp1, readfds, writefds, exceptfds, timeout_ptr, sigmask
    );

    // limit max fdp1
    max_fdp1 = cmp::min(max_fdp1, 255);

    let timeout = if timeout_ptr.is_valid() {
        let timeout = timeout_ptr.get_mut();
        debug!("[task {}] timeout: {:?}", user_task.get_task_id(), timeout);
        current_nsec() + timeout.to_nsec()
    } else {
        usize::MAX
    };
    let mut rfds_r = [0usize; 4];
    let mut wfds_r = [0usize; 4];
    let mut efds_r = [0usize; 4];
    loop {
        yield_now().await;
        let mut num = 0;
        let inner = user_task.pcb.lock();
        if readfds.is_valid() {
            let rfds = readfds.slice_mut_with_len(4);
            for i in 0..max_fdp1 {
                // iprove it
                if !rfds.get_bit(i) {
                    rfds_r.set_bit(i, false);
                    continue;
                }
                if inner.fd_table[i].is_none() {
                    rfds_r.set_bit(i, false);
                    continue;
                }
                let file = inner.fd_table[i].clone().unwrap();
                match file.poll(PollEvent::POLLIN) {
                    Ok(res) => {
                        if res.contains(PollEvent::POLLIN) {
                            num += 1;
                            rfds_r.set_bit(i, true);
                        } else {
                            rfds_r.set_bit(i, false)
                        }
                    }
                    Err(_) => rfds_r.set_bit(i, false),
                }
            }
        }
        if writefds.is_valid() {
            let wfds = writefds.slice_mut_with_len(4);
            for i in 0..max_fdp1 {
                if !wfds.get_bit(i) {
                    continue;
                }
                if inner.fd_table[i].is_none() {
                    wfds_r.set_bit(i, false);
                    continue;
                }
                let file = inner.fd_table[i].clone().unwrap();
                match file.poll(PollEvent::POLLOUT) {
                    Ok(res) => {
                        if res.contains(PollEvent::POLLOUT) {
                            num += 1;
                            wfds_r.set_bit(i, true);
                        } else {
                            wfds_r.set_bit(i, false);
                        }
                    }
                    Err(_) => wfds_r.set_bit(i, false),
                }
            }
        }
        if exceptfds.is_valid() {
            let efds = exceptfds.slice_mut_with_len(4);
            for i in 0..max_fdp1 {
                // iprove it
                if !efds.get_bit(i) {
                    continue;
                }
                if inner.fd_table[i].is_none() {
                    efds_r.set_bit(i, false);
                    continue;
                }
                let file = inner.fd_table[i].clone().unwrap();
                match file.poll(PollEvent::POLLERR) {
                    Ok(res) => {
                        if res.contains(PollEvent::POLLERR) {
                            num += 1;
                            efds_r.set_bit(i, true);
                        } else {
                            efds_r.set_bit(i, false)
                        }
                    }
                    Err(_) => efds_r.set_bit(i, false),
                }
            }
        }
        drop(inner);
        if num != 0 {
            if readfds.is_valid() {
                readfds.slice_mut_with_len(4).copy_from_slice(&rfds_r);
            }
            if writefds.is_valid() {
                writefds.slice_mut_with_len(4).copy_from_slice(&wfds_r);
            }
            if exceptfds.is_valid() {
                exceptfds.slice_mut_with_len(4).copy_from_slice(&efds_r);
            }
            return Ok(num);
        }

        if current_nsec() > timeout {
            if readfds.is_valid() {
                readfds.slice_mut_with_len(4).copy_from_slice(&rfds_r);
            }
            if writefds.is_valid() {
                writefds.slice_mut_with_len(4).copy_from_slice(&wfds_r);
            }
            if exceptfds.is_valid() {
                exceptfds.slice_mut_with_len(4).copy_from_slice(&efds_r);
            }
            return Ok(0);
        }
    }
}

pub async fn sys_ftruncate(fields: usize, len: usize) -> Result<usize, LinuxError> {
    debug!("sys_ftruncate @ fields: {}, len: {}", fields, len);
    // Ok(0)
    if fields == usize::MAX {
        return Err(LinuxError::EPERM);
    }
    let file = current_user_task()
        .get_fd(fields)
        .ok_or(LinuxError::EINVAL)?;
    file.truncate(len).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_epoll_create1(flags: usize) -> Result<usize, LinuxError> {
    debug!("sys_epoll_create @ flags: {:#x}", flags);
    let file = EpollFile::new(flags);
    let task = current_user_task();
    let fd = task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    task.set_fd(
        fd,
        Some(FileItem::new(Arc::new(file), FileOptions::default())),
    );
    Ok(fd)
}

pub async fn sys_epoll_ctl(
    epfd: usize,
    op: usize,
    fd: usize,
    event: UserRef<EpollEvent>,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_epoll_ctl @ epfd: {:#x} op: {:#x} fd: {:#x} event: {:#x?}",
        epfd, op, fd, event
    );
    let ctl = FromPrimitive::from_usize(op).ok_or(LinuxError::EINVAL)?;
    let task = current_user_task();
    let epfile = task
        .get_fd(epfd)
        .ok_or(LinuxError::EBADF)?
        .inner
        .downcast_arc::<EpollFile>()
        .map_err(|_| LinuxError::EINVAL)?;
    task.get_fd(fd).ok_or(LinuxError::EBADF)?;
    epfile.ctl(ctl, fd, event.get_ref().clone());
    Ok(0)
}

pub async fn sys_epoll_wait(
    epfd: usize,
    events: UserRef<EpollEvent>,
    max_events: usize,
    timeout: usize,
    sigmask: usize,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!("[task {}]sys_epoll_wait @ epfd: {:#x}, events: {:#x?}, max events: {:#x}, timeout: {:#x}, sigmask: {:#x}", task.get_task_id(), epfd, events, max_events, timeout, sigmask);
    let epfile = task
        .get_fd(epfd)
        .ok_or(LinuxError::EBADF)?
        .inner
        .downcast_arc::<EpollFile>()
        .map_err(|_| LinuxError::EINVAL)?;
    let stime = current_nsec();
    let end = stime + timeout * 0x1000_000;
    let buffer = events.slice_mut_with_len(max_events);
    let n = loop {
        let mut num = 0;
        for (fd, ev) in epfile.data.lock().iter() {
            if let Some(file) = task.get_fd(*fd) {
                if let Ok(pevent) = file.poll(ev.events.to_poll()) {
                    if pevent != PollEvent::NONE {
                        buffer[num] = ev.clone();
                        num += 1;
                    }
                }
            }
        }
        if current_nsec() >= end || num > 0 {
            break num;
        }
        yield_now().await;
    };

    Ok(n)
}
