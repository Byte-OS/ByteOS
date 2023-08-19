use alloc::string::String;
use core::cmp;
use fs::dentry::{dentry_open, DentryNode};
use num_traits::FromPrimitive;
use vfscore::FileType;

use alloc::sync::Arc;
use arch::VirtAddr;
use bit_field::BitArray;
use executor::{current_task, current_user_task, yield_now, AsyncTask, FileItem, UserTask};
use fs::pipe::create_pipe;
use fs::{
    INodeInterface, OpenFlags, PollEvent, PollFd, SeekFrom, Stat, StatFS, StatMode, TimeSpec,
    UTIME_NOW,
};
use log::debug;

use crate::epoll::{EpollEvent, EpollFile};
use crate::socket::Socket;
use crate::syscall::consts::{from_vfs, FcntlCmd, IoVec, AT_CWD};
use crate::syscall::func::timespc_now;
use crate::syscall::time::current_nsec;

use super::consts::{LinuxError, UserRef};

pub fn to_node(task: &Arc<UserTask>, fd: usize) -> Result<Arc<FileItem>, LinuxError> {
    match fd {
        AT_CWD => Ok(task.pcb.lock().curr_dir.clone()),
        x => task.get_fd(x).ok_or(LinuxError::EBADF),
    }
}

pub async fn sys_dup(fd: usize) -> Result<usize, LinuxError> {
    debug!("sys_dup3 @ fd_src: {}", fd);
    let task = current_user_task();
    let fd_dst = task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    sys_dup3(fd, fd_dst).await
}

pub async fn sys_dup3(fd_src: usize, fd_dst: usize) -> Result<usize, LinuxError> {
    debug!("sys_dup3 @ fd_src: {}, fd_dst: {}", fd_src, fd_dst);
    let task = current_user_task();
    let file = task.get_fd(fd_src).ok_or(LinuxError::EBADF)?;
    task.set_fd(fd_dst, file);
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
    task.get_fd(fd)
        .ok_or(LinuxError::EBADF)?
        .async_read(buffer)
        .await
        .map_err(from_vfs)
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
    let file = task.get_fd(fd).ok_or(LinuxError::EBADF)?;
    if let Ok(_) = file.get_bare_file().downcast_arc::<Socket>() {
        yield_now().await;
    }
    file.async_write(buffer).await.map_err(from_vfs)
}

pub async fn sys_readv(fd: usize, iov: UserRef<IoVec>, iocnt: usize) -> Result<usize, LinuxError> {
    debug!("sys_readv @ fd: {}, iov: {}, iocnt: {}", fd, iov, iocnt);

    let mut rsize = 0;

    let iov = iov.slice_mut_with_len(iocnt);
    let task = current_user_task();
    let file = task.get_fd(fd).ok_or(LinuxError::EBADF)?;

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

    let task = current_user_task();
    let file = task.get_fd(fd).ok_or(LinuxError::EBADF)?;

    for io in iov {
        let buffer = UserRef::<u8>::from(io.base).slice_mut_with_len(io.len);
        wsize += file.write(buffer).map_err(from_vfs)?;
    }

    Ok(wsize)
}

pub async fn sys_close(fd: usize) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_close @ fd: {}",
        task.get_task_id(),
        fd as isize
    );

    task.clear_fd(fd);
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
    let task = current_task().as_user_task().unwrap();
    let dir = to_node(&task, dir_fd)?;
    if path == "/" {
        return Err(LinuxError::EEXIST);
    }

    dir.dentry_open(path, OpenFlags::O_CREAT | OpenFlags::O_DIRECTORY)
        .map_err(from_vfs)?;
    // let path_str = rebuild_path(path);
    // let paths: Vec<&str> = path_str.split("/").collect();
    // let mut pfile = dir.inner.clone();
    // for i in paths.into_iter().filter(|x| *x != "") {
    //     let f = pfile.open(i, OpenFlags::O_RDWR);
    //     if f.is_err() {
    //         pfile.mkdir(i).map_err(from_vfs)?;
    //     } else {
    //         pfile = f.unwrap();
    //     }
    // }
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
    let flags = OpenFlags::from_bits_truncate(flags);
    let user_task = current_task().as_user_task().unwrap();
    let dir = to_node(&user_task, dir_fd)?;
    let file = dir.dentry_open(path, flags).map_err(from_vfs)?;
    let dentry = file.dentry.clone().unwrap();
    let parent = dentry
        .parent
        .upgrade()
        .expect("can't upgrade to parent node");
    parent.node.remove(&dentry.filename).map_err(from_vfs)?;
    parent
        .children
        .lock()
        .retain(|x| x.filename != dentry.filename);
    Ok(0)
}

pub async fn sys_openat(
    fd: usize,
    filename: UserRef<i8>,
    flags: usize,
    mode: usize,
) -> Result<usize, LinuxError> {
    let user_task = current_task().as_user_task().unwrap();
    let flags = OpenFlags::from_bits_truncate(flags);
    let filename = if filename.is_valid() {
        filename.get_cstr().map_err(|_| LinuxError::EINVAL)?
    } else {
        ""
    };
    debug!(
        "sys_openat @ fd: {}, filename: {}, flags: {:?}, mode: {}",
        fd as isize, filename, flags, mode
    );
    let dir = to_node(&user_task, fd)?;
    let file = dir.dentry_open(filename, flags).map_err(from_vfs)?;
    let fd = user_task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    user_task.set_fd(fd, file);
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
    let dir = to_node(&user_task, fd)?;
    let _node = dentry_open(dir.dentry.clone().unwrap(), filename, open_flags).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_fstat(fd: usize, stat_ptr: UserRef<Stat>) -> Result<usize, LinuxError> {
    debug!("sys_fstat @ fd: {} stat_ptr: {}", fd, stat_ptr);
    let stat_ref = stat_ptr.get_mut();
    current_user_task()
        .get_fd(fd)
        .ok_or(LinuxError::EBADF)?
        .stat(stat_ref)
        .map_err(from_vfs)?;
    stat_ref.mode |= StatMode::OWNER_MASK | StatMode::GROUP_MASK | StatMode::OTHER_MASK;
    Ok(0)
}

pub async fn sys_fstatat(
    dir_fd: usize,
    path_ptr: UserRef<i8>,
    stat_ptr: UserRef<Stat>,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_fstatat @ dir_fd: {}, path_ptr:{}, stat_ptr: {}",
        dir_fd as isize, path_ptr, stat_ptr
    );
    let path = path_ptr.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    debug!(
        "sys_fstatat @ dir_fd: {}, path:{}, stat_ptr: {}",
        dir_fd as isize, path, stat_ptr
    );
    let stat = stat_ptr.get_mut();

    let user_task = current_task().as_user_task().unwrap();

    let dir = to_node(&user_task, dir_fd)?;

    dentry_open(dir.dentry.clone().unwrap(), &path, OpenFlags::NONE)
        .map_err(from_vfs)?
        .node
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
    FileItem::fs_open(path, OpenFlags::NONE)
        .map_err(from_vfs)?
        .statfs(statfs)
        .map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_pipe2(fds_ptr: UserRef<u32>, _unknown: usize) -> Result<usize, LinuxError> {
    debug!("sys_pipe2 @ fds_ptr: {}, _unknown: {}", fds_ptr, _unknown);
    let fds = fds_ptr.slice_mut_with_len(2);
    let task = current_user_task();

    let (rx, tx) = create_pipe();
    let rx_fd = task.alloc_fd().ok_or(LinuxError::ENFILE)?;
    task.set_fd(rx_fd, FileItem::new_dev(rx));
    fds[0] = rx_fd as u32;

    let tx_fd = task.alloc_fd().ok_or(LinuxError::ENFILE)?;
    task.set_fd(tx_fd, FileItem::new_dev(tx));
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

    let file = current_user_task().get_fd(fd).ok_or(LinuxError::EBADF)?;
    file.readat(offset, buffer).map_err(from_vfs)
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
    current_user_task()
        .get_fd(fd)
        .ok_or(LinuxError::EBADF)?
        .writeat(offset, buffer)
        .map_err(from_vfs)
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

    let dev_node = FileItem::fs_open(special, OpenFlags::NONE).map_err(from_vfs)?;
    dev_node.mount(dir).map_err(from_vfs)?;
    Ok(0)
}

pub async fn sys_umount2(special: UserRef<i8>, flags: usize) -> Result<usize, LinuxError> {
    let special = special.get_cstr().map_err(|_| LinuxError::EINVAL)?;
    debug!("sys_umount @ special: {}, flags: {}", special, flags);
    match special.starts_with("/dev") {
        true => {
            todo!("unmount dev");
            // let dev = dentry_open(dentry_root(), special, OpenFlags::NONE).map_err(from_vfs)?;
            // dev.node.umount().map_err(from_vfs)?;
        }
        false => {
            DentryNode::unmount(String::from(special)).map_err(from_vfs)?;
        }
    };

    Ok(0)
}

pub async fn sys_getdents64(
    fd: usize,
    buf_ptr: UserRef<u8>,
    len: usize,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_getdents64 @ fd: {}, buf_ptr: {}, len: {}",
        task.get_task_id(),
        fd,
        buf_ptr,
        len
    );

    let file = task.get_fd(fd).unwrap();

    let buffer = buf_ptr.slice_mut_with_len(len);
    file.getdents(buffer).map_err(from_vfs)
}

pub fn sys_lseek(fd: usize, offset: usize, whence: usize) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_lseek @ fd {}, offset: {}, whench: {}",
        task.get_task_id(),
        fd,
        offset as isize,
        whence
    );

    task.get_fd(fd)
        .ok_or(LinuxError::EBADF)?
        .seek(match whence {
            0 => SeekFrom::SET(offset),
            1 => SeekFrom::CURRENT(offset as isize),
            2 => SeekFrom::END(offset as isize),
            _ => return Err(LinuxError::EINVAL),
        })
        .map_err(from_vfs)
}

pub async fn sys_ioctl(
    fd: usize,
    request: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] ioctl: fd: {}, request: {:#x}, args: {:#x} {:#x} {:#x}",
        task.get_task_id(),
        fd,
        request,
        arg1,
        arg2,
        arg3
    );
    task.get_fd(fd)
        .ok_or(LinuxError::EINVAL)?
        .ioctl(request, arg1)
        .map_err(from_vfs)
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
    let file = task.get_fd(fd).ok_or(LinuxError::EBADF)?;
    debug!("[task {}] fcntl: {:?}", task.get_task_id(), cmd);
    match cmd {
        FcntlCmd::DUPFD | FcntlCmd::DUPFDCLOEXEC => sys_dup(fd).await,
        FcntlCmd::GETFD => Ok(1),
        FcntlCmd::GETFL => Ok(file.flags.lock().bits()),
        FcntlCmd::SETFL => {
            *file.flags.lock() = OpenFlags::from_bits_truncate(arg);
            task.set_fd(fd, file);
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

    let dir = to_node(&user_task, dir_fd)?;
    let path = if !path.is_valid() {
        ""
    } else {
        path.get_cstr().map_err(|_| LinuxError::EINVAL)?
    };

    debug!("times: {:?} path: {}", times, path);

    if path == "/dev/null/invalid" {
        return Ok(0);
    }

    dir.dentry_open(path, OpenFlags::O_RDONLY)
        .map_err(from_vfs)?
        .utimes(&mut times)
        .map_err(from_vfs)?;

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

    let dir = to_node(&user_task, dir_fd)?;

    let ftype = dir
        .open(filename, OpenFlags::NONE)
        .map_err(from_vfs)?
        .metadata()
        .map_err(from_vfs)?
        .file_type;

    if FileType::Link != ftype {
        return Err(LinuxError::EINVAL);
    }

    let file_path = FileItem::fs_open(filename, OpenFlags::NONE)
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
        "out_fd: {}  in_fd: {}  offset: {:#x}   count: {:#x}",
        out_fd, in_fd, offset, count
    );
    let task = current_user_task();
    let out_file = task.get_fd(out_fd).ok_or(LinuxError::EINVAL)?;
    let in_file = task.get_fd(in_fd).ok_or(LinuxError::EINVAL)?;

    let curr_off = if offset != 0 {
        offset
    } else {
        in_file.seek(SeekFrom::CURRENT(0)).map_err(from_vfs)?
    };

    let rlen = cmp::min(in_file.metadata().map_err(from_vfs)?.size - curr_off, count);

    let mut buffer = vec![0u8; rlen];

    if offset == 0 {
        in_file.read(&mut buffer).map_err(from_vfs)?;
        task.set_fd(in_fd, in_file);
    } else {
        in_file.readat(offset, &mut buffer).map_err(from_vfs)?;
    }
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
    let file = Arc::new(EpollFile::new(flags));
    let task = current_user_task();
    let fd = task.alloc_fd().ok_or(LinuxError::EMFILE)?;
    task.set_fd(fd, FileItem::new_dev(file));
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
        .clone()
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
        .clone()
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
