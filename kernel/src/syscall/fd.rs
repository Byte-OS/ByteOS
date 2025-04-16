use super::types::fd::IoVec;
use super::types::poll::{EpollEvent, EpollFile};
use super::SysResult;
use crate::syscall::types::fd::{FcntlCmd, AT_CWD};
use crate::user::UserTaskContainer;
use crate::utils::time::{current_nsec, current_timespec};
use crate::utils::useref::UserRef;
use alloc::sync::Arc;
use bit_field::BitArray;
use core::cmp;
use executor::yield_now;
use fs::dentry::umount;
use fs::file::File;
use fs::{
    pipe::create_pipe, OpenFlags, PollEvent, PollFd, SeekFrom, Stat, StatFS, StatMode, TimeSpec,
    UTIME_NOW,
};
use log::debug;
use num_traits::FromPrimitive;
use polyhal::VirtAddr;
use syscalls::Errno;
use vfscore::FileType;

impl UserTaskContainer {
    pub async fn sys_dup(&self, fd: usize) -> SysResult {
        debug!("sys_dup3 @ fd_src: {}", fd);
        let fd_dst = self.task.alloc_fd().ok_or(Errno::EMFILE)?;
        self.sys_dup3(fd, fd_dst).await
    }

    pub async fn sys_dup3(&self, fd_src: usize, fd_dst: usize) -> SysResult {
        debug!("sys_dup3 @ fd_src: {}, fd_dst: {}", fd_src, fd_dst);
        let file = self.task.get_fd(fd_src).ok_or(Errno::EBADF)?;
        self.task.set_fd(fd_dst, file);
        Ok(fd_dst)
    }

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_dup2(&self, fd_src: usize, fd_dst: usize) -> SysResult {
        self.sys_dup3(fd_src, fd_dst).await
    }

    pub async fn sys_read(&self, fd: usize, buf_ptr: UserRef<u8>, count: usize) -> SysResult {
        debug!(
            "[task {}] sys_read @ fd: {} buf_ptr: {:?} count: {}",
            self.tid, fd as isize, buf_ptr, count
        );
        let buffer = buf_ptr.slice_mut_with_len(count);
        self.task
            .get_fd(fd)
            .ok_or(Errno::EBADF)?
            .async_read(buffer)
            .await
    }

    pub async fn sys_write(&self, fd: usize, buf_ptr: VirtAddr, count: usize) -> SysResult {
        debug!(
            "[task {}] sys_write @ fd: {} buf_ptr: {:?} count: {}",
            self.tid, fd as isize, buf_ptr, count
        );
        let buffer = buf_ptr.slice_with_len(count);
        let file = self.task.get_fd(fd).ok_or(Errno::EBADF)?;
        // if let Ok(_) = file.get_bare_file().downcast_arc::<Socket>() {
        //     yield_now().await;
        // }
        file.async_write(buffer).await
    }

    pub async fn sys_readv(&self, fd: usize, iov: UserRef<IoVec>, iocnt: usize) -> SysResult {
        debug!("sys_readv @ fd: {}, iov: {}, iocnt: {}", fd, iov, iocnt);

        let mut rsize = 0;

        let iov = iov.slice_mut_with_len(iocnt);
        let file = self.task.get_fd(fd).ok_or(Errno::EBADF)?;

        for io in iov {
            let buffer = UserRef::<u8>::from(io.base).slice_mut_with_len(io.len);
            rsize += file.read(buffer)?;
        }

        Ok(rsize)
    }

    pub async fn sys_writev(&self, fd: usize, iov: UserRef<IoVec>, iocnt: usize) -> SysResult {
        debug!("sys_writev @ fd: {}, iov: {}, iocnt: {}", fd, iov, iocnt);
        let mut wsize = 0;

        let iov = iov.slice_mut_with_len(iocnt);

        let file = self.task.get_fd(fd).ok_or(Errno::EBADF)?;

        for io in iov {
            let buffer = UserRef::<u8>::from(io.base).slice_mut_with_len(io.len);
            wsize += file.write(buffer)?;
        }

        Ok(wsize)
    }

    pub async fn sys_close(&self, fd: usize) -> SysResult {
        debug!("[task {}] sys_close @ fd: {}", self.tid, fd as isize);

        self.task.clear_fd(fd);
        Ok(0)
    }

    pub async fn sys_mkdir_at(&self, dir_fd: isize, path: UserRef<i8>, mode: usize) -> SysResult {
        let path = path.get_cstr().map_err(|_| Errno::EINVAL)?;
        debug!(
            "sys_mkdir_at @ dir_fd: {}, path: {}, mode: {}",
            dir_fd as isize, path, mode
        );
        self.task
            .fd_open(dir_fd, path, OpenFlags::O_DIRECTORY | OpenFlags::O_CREAT)?;
        Ok(0)
    }

    pub async fn sys_renameat2(
        &self,
        olddir_fd: isize,
        oldpath: UserRef<i8>,
        newdir_fd: isize,
        newpath: UserRef<i8>,
        flags: usize,
    ) -> SysResult {
        debug!(
            "sys_renameat2 @ olddir_fd: {}, oldpath: {}, newdir_fd: {}, newpath: {}, flags: {}",
            olddir_fd, oldpath, newdir_fd, newpath, flags
        );
        let flags = OpenFlags::from_bits_truncate(flags);

        let old_path: &str = oldpath.get_cstr().map_err(|_| Errno::EINVAL)?;
        let old_file = self.task.fd_open(olddir_fd, old_path, flags.clone())?;

        let old_file_type = old_file.file_type()?;
        let new_path = newpath.get_cstr().map_err(|_| Errno::EINVAL)?;

        if old_file_type == FileType::File {
            let new_file = self
                .task
                .fd_open(newdir_fd, new_path, OpenFlags::O_CREAT | flags)?;
            let file_size = old_file.file_size()?;
            let mut buffer = vec![0u8; file_size];
            old_file.read(&mut buffer)?;
            new_file.write(&buffer)?;
            new_file.truncate(buffer.len())?;
        } else if old_file_type == FileType::Directory {
            self.task.fd_open(
                newdir_fd,
                new_path,
                OpenFlags::O_CREAT | OpenFlags::O_DIRECTORY | flags,
            )?;
        } else {
            panic!("can't handle the file: {:?} now", old_file_type);
        }

        Ok(0)
    }

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_mkdir(&self, path: UserRef<i8>, mode: usize) -> SysResult {
        self.sys_mkdir_at(AT_CWD, path, mode).await
    }

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_unlink(&self, path: UserRef<i8>) -> SysResult {
        self.sys_unlinkat(AT_CWD, path, 0).await
    }

    pub async fn sys_unlinkat(&self, dir_fd: isize, path: UserRef<i8>, flags: usize) -> SysResult {
        let path = path.get_cstr().map_err(|_| Errno::EINVAL)?;
        debug!(
            "sys_unlinkat @ dir_fd: {}, path: {}, flags: {}",
            dir_fd as isize, path, flags
        );
        let flags = OpenFlags::from_bits_truncate(flags);
        let file = self.task.fd_open(dir_fd, path, flags)?;

        file.remove_self()?;
        Ok(0)
    }

    pub async fn sys_openat(
        &self,
        dir_fd: isize,
        filename: UserRef<i8>,
        flags: usize,
        mode: usize,
    ) -> SysResult {
        let flags = OpenFlags::from_bits_truncate(flags);
        let filename = if filename.is_valid() {
            filename.get_cstr().map_err(|_| Errno::EINVAL)?
        } else {
            ""
        };
        debug!(
            "sys_openat @ fd: {}, filename: {}, flags: {:?}, mode: {}",
            dir_fd as isize, filename, flags, mode
        );
        // let dir = to_node(&self.task, fd, filename)?;
        // let file = dir.dentry_open(filename, flags)?;
        let file = self.task.fd_open(dir_fd, filename, flags)?;
        let fd = self.task.alloc_fd().ok_or(Errno::EMFILE)?;
        self.task.set_fd(fd, Arc::new(file));
        debug!("sys_openat @ ret fd: {}", fd);
        Ok(fd)
    }

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_open(&self, path: UserRef<i8>, flags: usize, mode: usize) -> SysResult {
        // syscall_openat(axprocess::link::AT_FDCWD, path, flags, mode)
        self.sys_openat(AT_CWD, path, flags, mode).await
    }

    pub async fn sys_faccess_at(
        &self,
        dir_fd: isize,
        filename: UserRef<i8>,
        mode: usize,
        flags: usize,
    ) -> SysResult {
        let open_flags = OpenFlags::from_bits_truncate(flags);
        let filename = if filename.is_valid() {
            filename.get_cstr().map_err(|_| Errno::EINVAL)?
        } else {
            ""
        };
        debug!(
            "sys_accessat @ fd: {}, filename: {}, flags: {:?}, mode: {}",
            dir_fd as isize, filename, open_flags, mode
        );
        self.task.fd_open(dir_fd, filename, open_flags)?;
        Ok(0)
    }

    pub async fn sys_fstat(&self, fd: usize, stat_ptr: UserRef<Stat>) -> SysResult {
        debug!("sys_fstat @ fd: {} stat_ptr: {}", fd, stat_ptr);
        let stat_ref = stat_ptr.get_mut();

        let file = self.task.get_fd(fd).ok_or(Errno::EBADF)?;
        file.stat(stat_ref)?;
        stat_ref.mode |= StatMode::OWNER_MASK;
        Ok(0)
    }

    pub async fn sys_fstatat(
        &self,
        dir_fd: isize,
        path_ptr: UserRef<i8>,
        stat_ptr: UserRef<Stat>,
    ) -> SysResult {
        debug!(
            "sys_fstatat @ dir_fd: {}, path_ptr:{}, stat_ptr: {}",
            dir_fd as isize, path_ptr, stat_ptr
        );
        let path = path_ptr.get_cstr().map_err(|_| Errno::EINVAL)?;
        debug!(
            "sys_fstatat @ dir_fd: {}, path:{}, stat_ptr: {}",
            dir_fd as isize, path, stat_ptr
        );
        let stat = stat_ptr.get_mut();

        self.task
            .fd_open(dir_fd, path, OpenFlags::O_RDONLY)?
            .stat(stat)?;
        stat.mode |= StatMode::OWNER_MASK;
        Ok(0)
    }

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_stat(&self, path: UserRef<i8>, stat_ptr: UserRef<Stat>) -> SysResult {
        self.sys_fstatat(AT_CWD, path, stat_ptr).await
    }

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_lstat(&self, path: UserRef<i8>, stat_ptr: UserRef<Stat>) -> SysResult {
        self.sys_fstatat(AT_CWD, path, stat_ptr).await
    }

    pub async fn sys_statfs(
        &self,
        filename_ptr: UserRef<i8>,
        statfs_ptr: UserRef<StatFS>,
    ) -> SysResult {
        debug!(
            "sys_statfs @ filename_ptr: {}, statfs_ptr: {}",
            filename_ptr, statfs_ptr
        );
        let path = filename_ptr.get_cstr().map_err(|_| Errno::EINVAL)?;
        let statfs = statfs_ptr.get_mut();
        File::open(path, OpenFlags::O_RDONLY)?.statfs(statfs)?;
        Ok(0)
    }

    pub async fn sys_pipe2(&self, fds_ptr: UserRef<u32>, _unknown: usize) -> SysResult {
        debug!("sys_pipe2 @ fds_ptr: {}, _unknown: {}", fds_ptr, _unknown);
        let fds = fds_ptr.slice_mut_with_len(2);

        let (rx, tx) = create_pipe();
        let rx_fd = self.task.alloc_fd().ok_or(Errno::ENFILE)?;
        self.task.set_fd(rx_fd, File::new_dev(rx));
        fds[0] = rx_fd as u32;

        let tx_fd = self.task.alloc_fd().ok_or(Errno::ENFILE)?;
        self.task.set_fd(tx_fd, File::new_dev(tx));
        fds[1] = tx_fd as u32;

        debug!("sys_pipe2 ret: {} {}", rx_fd as u32, tx_fd as u32);
        Ok(0)
    }

    pub async fn sys_pread(
        &self,
        fd: usize,
        ptr: UserRef<u8>,
        len: usize,
        offset: usize,
    ) -> SysResult {
        debug!(
            "sys_pread @ fd: {}, ptr: {}, len: {}, offset: {}",
            fd, ptr, len, offset
        );
        let buffer = ptr.slice_mut_with_len(len);

        let file = self.task.get_fd(fd).ok_or(Errno::EBADF)?;
        file.readat(offset, buffer)
    }

    pub async fn sys_pwrite(
        &self,
        fd: usize,
        buf_ptr: VirtAddr,
        count: usize,
        offset: usize,
    ) -> SysResult {
        debug!(
            "sys_write @ fd: {} buf_ptr: {:?} count: {}",
            fd as isize, buf_ptr, count
        );
        let buffer = buf_ptr.slice_with_len(count);
        self.task
            .get_fd(fd)
            .ok_or(Errno::EBADF)?
            .writeat(offset, buffer)
    }

    pub async fn sys_mount(
        &self,
        special: UserRef<i8>,
        dir: UserRef<i8>,
        fstype: UserRef<i8>,
        flags: usize,
        data: usize,
    ) -> SysResult {
        let special = special.get_cstr().map_err(|_| Errno::EINVAL)?;
        let dir = dir.get_cstr().map_err(|_| Errno::EINVAL)?;
        let fstype = fstype.get_cstr().map_err(|_| Errno::EINVAL)?;
        debug!(
            "sys_mount @ special: {}, dir: {}, fstype: {}, flags: {}, data: {:#x}",
            special, dir, fstype, flags, data
        );

        let dev_node = File::open(special, OpenFlags::NONE)?;
        dev_node.mount(dir)?;
        Ok(0)
    }

    pub async fn sys_umount2(&self, special: UserRef<i8>, flags: usize) -> SysResult {
        let special = special.get_cstr().map_err(|_| Errno::EINVAL)?;
        debug!("sys_umount @ special: {}, flags: {}", special, flags);
        match special.starts_with("/dev") {
            true => {
                todo!("unmount dev");
                // let dev = dentry_open(dentry_root(), special, OpenFlags::NONE).map_err(from_vfs)?;
                // dev.node.umount().map_err(from_vfs)?;
            }
            false => umount(special)?,
        };

        Ok(0)
    }

    pub async fn sys_getdents64(&self, fd: usize, buf_ptr: UserRef<u8>, len: usize) -> SysResult {
        debug!(
            "[task {}] sys_getdents64 @ fd: {}, buf_ptr: {}, len: {}",
            self.tid, fd, buf_ptr, len
        );

        let file = self.task.get_fd(fd).unwrap();

        let buffer = buf_ptr.slice_mut_with_len(len);
        file.getdents(buffer)
    }

    pub fn sys_lseek(&self, fd: usize, offset: usize, whence: usize) -> SysResult {
        debug!(
            "[task {}] sys_lseek @ fd {}, offset: {}, whench: {}",
            self.tid, fd, offset as isize, whence
        );

        let seek_from = match whence {
            0 => SeekFrom::SET(offset),
            1 => SeekFrom::CURRENT(offset as isize),
            2 => SeekFrom::END(offset as isize),
            _ => return Err(Errno::EINVAL),
        };

        self.task.get_fd(fd).ok_or(Errno::EBADF)?.seek(seek_from)
    }

    pub async fn sys_ioctl(
        &self,
        fd: usize,
        request: usize,
        arg1: usize,
        arg2: usize,
        arg3: usize,
    ) -> SysResult {
        debug!(
            "[task {}] ioctl: fd: {}, request: {:#x}, args: {:#x} {:#x} {:#x}",
            self.tid, fd, request, arg1, arg2, arg3
        );
        self.task
            .get_fd(fd)
            .ok_or(Errno::EINVAL)?
            .ioctl(request, arg1)
            .map_err(|_| Errno::ENOTTY)
    }

    pub async fn sys_fcntl(&self, fd: usize, cmd: usize, arg: usize) -> SysResult {
        debug!(
            "[task {}] fcntl: fd: {}, cmd: {:#x}, arg: {}",
            self.tid, fd, cmd, arg
        );
        let cmd = FromPrimitive::from_usize(cmd).ok_or(Errno::EINVAL)?;
        let file = self.task.get_fd(fd).ok_or(Errno::EBADF)?;
        debug!("[task {}] fcntl: {:?}", self.tid, cmd);
        match cmd {
            FcntlCmd::DUPFD | FcntlCmd::DUPFDCLOEXEC => self.sys_dup(fd).await,
            FcntlCmd::GETFD => Ok(1),
            FcntlCmd::GETFL => Ok(file.flags.lock().bits()),
            FcntlCmd::SETFL => {
                *file.flags.lock() = OpenFlags::from_bits_truncate(arg);
                self.task.set_fd(fd, file);
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
        &self,
        dir_fd: isize,
        path: UserRef<u8>,
        times_ptr: UserRef<TimeSpec>,
        flags: usize,
    ) -> SysResult {
        debug!(
            "sys_utimensat @ dir_fd: {}, path: {}, times_ptr: {}, flags: {}",
            dir_fd, path, times_ptr, flags
        );
        // build times
        let mut times = match !times_ptr.is_valid() {
            true => {
                vec![current_timespec(), current_timespec()]
            }
            false => {
                let ts = times_ptr.slice_mut_with_len(2);
                let mut times = vec![];
                for i in 0..2 {
                    if ts[i].nsec == UTIME_NOW {
                        times.push(current_timespec());
                    } else {
                        times.push(ts[i]);
                    }
                }
                times
            }
        };

        let path = if !path.is_valid() {
            ""
        } else {
            path.get_cstr().map_err(|_| Errno::EINVAL)?
        };

        debug!("times: {:?} path: {}", times, path);

        if path == "/dev/null/invalid" {
            return Ok(0);
        }

        self.task
            .fd_open(dir_fd, path, OpenFlags::O_RDONLY)?
            .utimes(&mut times)?;

        Ok(0)
    }

    pub async fn sys_readlinkat(
        &self,
        dir_fd: isize,
        path: UserRef<i8>,
        buffer: UserRef<u8>,
        buffer_size: usize,
    ) -> SysResult {
        debug!(
            "sys_readlinkat @ dir_fd: {}, path: {}, buffer: {}, size: {}",
            dir_fd, path, buffer, buffer_size
        );
        let filename = path.get_cstr().map_err(|_| Errno::EINVAL)?;
        let buffer = buffer.slice_mut_with_len(buffer_size);
        debug!("readlinkat @ filename: {}", filename);

        let ftype = self
            .task
            .fd_open(dir_fd, filename, OpenFlags::O_RDONLY)?
            .file_type()?;

        if FileType::Link != ftype {
            return Err(Errno::EINVAL);
        }

        let file_path = File::open(filename, OpenFlags::NONE)?.resolve_link()?;
        let bytes = file_path.as_bytes();

        let rlen = cmp::min(bytes.len(), buffer_size);

        buffer[..rlen].copy_from_slice(&bytes[..rlen]);
        debug!("sys_readlinkat: rlen: {}", rlen);
        Ok(rlen)
    }

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_readlink(
        &self,
        path: UserRef<i8>,
        buffer: UserRef<u8>,
        buffer_size: usize,
    ) -> SysResult {
        self.sys_readlinkat(AT_CWD, path, buffer, buffer_size).await
    }

    pub async fn sys_sendfile(
        &self,
        out_fd: usize,
        in_fd: usize,
        offset: usize,
        count: usize,
    ) -> SysResult {
        debug!(
            "sys_sendfile @ out_fd: {}  in_fd: {}  offset: {:#x}   count: {:#x}",
            out_fd, in_fd, offset, count
        );
        let out_file = self.task.get_fd(out_fd).ok_or(Errno::EINVAL)?;
        let in_file = self.task.get_fd(in_fd).ok_or(Errno::EINVAL)?;

        let curr_off = if offset != 0 {
            offset
        } else {
            in_file.seek(SeekFrom::CURRENT(0))?
        };
        let rlen = cmp::min(in_file.file_size()? - curr_off, count);

        let mut buffer = vec![0u8; rlen];

        if offset == 0 {
            in_file.read(&mut buffer)?;
            self.task.set_fd(in_fd, in_file);
        } else {
            in_file.readat(offset, &mut buffer)?;
        }
        out_file.write(&buffer)
    }

    /// TODO: improve it.
    pub async fn sys_ppoll(
        &self,
        poll_fds_ptr: UserRef<PollFd>,
        nfds: usize,
        timeout_ptr: UserRef<TimeSpec>,
        sigmask_ptr: usize,
    ) -> SysResult {
        debug!(
            "sys_ppoll @ poll_fds_ptr: {}, nfds: {}, timeout_ptr: {}, sigmask_ptr: {:#X}",
            poll_fds_ptr, nfds, timeout_ptr, sigmask_ptr
        );
        let poll_fds = poll_fds_ptr.slice_mut_with_len(nfds);
        let etime = if timeout_ptr.is_valid() {
            current_nsec() + timeout_ptr.get_ref().to_nsec()
        } else {
            usize::MAX
        };
        let n = loop {
            let mut num = 0;
            for i in 0..nfds {
                poll_fds[i].revents = self
                    .task
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

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_poll(
        &self,
        poll_fds_ptr: UserRef<PollFd>,
        nfds: usize,
        timeout: isize,
    ) -> SysResult {
        debug!(
            "sys_poll @ poll_fds_ptr: {}, nfds: {}, timeout: {}",
            poll_fds_ptr, nfds, timeout
        );
        let poll_fds = poll_fds_ptr.slice_mut_with_len(nfds);
        let etime = current_nsec() + timeout as usize * 0x1000_000;
        let n = loop {
            let mut num = 0;
            for i in 0..nfds {
                poll_fds[i].revents = self
                    .task
                    .get_fd(poll_fds[i].fd as _)
                    .map_or(PollEvent::NONE, |x| {
                        x.poll(poll_fds[i].events.clone()).unwrap()
                    });
                if poll_fds[i].revents != PollEvent::NONE {
                    num += 1;
                }
            }

            if (timeout > 0 && current_nsec() >= etime) || num > 0 {
                break num;
            }
            yield_now().await;
        };
        Ok(n)
    }

    /// TODO: improve it.
    pub async fn sys_pselect(
        &self,
        mut max_fdp1: usize,
        readfds: UserRef<usize>,
        writefds: UserRef<usize>,
        exceptfds: UserRef<usize>,
        timeout_ptr: UserRef<TimeSpec>,
        sigmask: usize,
    ) -> SysResult {
        debug!(
            "[task {}] sys_pselect @ max_fdp1: {}, readfds: {}, writefds: {}, exceptfds: {}, tsptr: {}, sigmask: {:#X}",
            self.tid, max_fdp1, readfds, writefds, exceptfds, timeout_ptr, sigmask
        );

        // limit max fdp1
        max_fdp1 = cmp::min(max_fdp1, 255);

        let timeout = if timeout_ptr.is_valid() {
            let timeout = timeout_ptr.get_mut();
            debug!("[task {}] timeout: {:?}", self.tid, timeout);
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
            let inner = self.task.pcb.lock();
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

    #[cfg(target_arch = "x86_64")]
    pub async fn sys_select(
        &self,
        max_fdp1: usize,
        readfds: UserRef<usize>,
        writefds: UserRef<usize>,
        exceptfds: UserRef<usize>,
        timeout_ptr: UserRef<TimeSpec>,
    ) -> SysResult {
        self.sys_pselect(max_fdp1, readfds, writefds, exceptfds, timeout_ptr, 0)
            .await
    }

    pub async fn sys_ftruncate(&self, fields: usize, len: usize) -> SysResult {
        debug!("sys_ftruncate @ fields: {}, len: {}", fields, len);
        // Ok(0)
        if fields == usize::MAX {
            return Err(Errno::EPERM);
        }
        let file = self.task.get_fd(fields).ok_or(Errno::EINVAL)?;
        file.truncate(len)?;
        Ok(0)
    }

    pub async fn sys_epoll_create1(&self, flags: usize) -> SysResult {
        debug!("sys_epoll_create @ flags: {:#x}", flags);
        let file = Arc::new(EpollFile::new(flags));
        let fd = self.task.alloc_fd().ok_or(Errno::EMFILE)?;
        self.task.set_fd(fd, File::new_dev(file));
        Ok(fd)
    }

    pub async fn sys_epoll_ctl(
        &self,
        epfd: usize,
        op: usize,
        fd: usize,
        event: UserRef<EpollEvent>,
    ) -> SysResult {
        debug!(
            "sys_epoll_ctl @ epfd: {:#x} op: {:#x} fd: {:#x} event: {:#x?}",
            epfd, op, fd, event
        );
        let ctl = FromPrimitive::from_usize(op).ok_or(Errno::EINVAL)?;
        let epfile = self
            .task
            .get_fd(epfd)
            .ok_or(Errno::EBADF)?
            .inner
            .clone()
            .downcast_arc::<EpollFile>()
            .map_err(|_| Errno::EINVAL)?;
        self.task.get_fd(fd).ok_or(Errno::EBADF)?;
        epfile.ctl(ctl, fd, event.get_ref().clone());
        Ok(0)
    }

    pub async fn sys_epoll_wait(
        &self,
        epfd: usize,
        events: UserRef<EpollEvent>,
        max_events: usize,
        timeout: usize,
        sigmask: usize,
    ) -> SysResult {
        debug!("[task {}]sys_epoll_wait @ epfd: {:#x}, events: {:#x?}, max events: {:#x}, timeout: {:#x}, sigmask: {:#x}", self.tid, epfd, events, max_events, timeout, sigmask);
        let epfile = self
            .task
            .get_fd(epfd)
            .ok_or(Errno::EBADF)?
            .inner
            .clone()
            .downcast_arc::<EpollFile>()
            .map_err(|_| Errno::EINVAL)?;
        let stime = current_nsec();
        let end = if timeout == usize::MAX {
            usize::MAX
        } else {
            stime + timeout * 0x1000_000
        };
        let buffer = events.slice_mut_with_len(max_events);
        debug!("epoll_wait:{:#x?}", epfile.data.lock());
        let n = loop {
            yield_now().await;
            let mut num = 0;
            for (fd, ev) in epfile.data.lock().iter() {
                if let Some(file) = self.task.get_fd(*fd) {
                    if let Ok(pevent) = file.poll(ev.events.to_poll()) {
                        if pevent != PollEvent::NONE {
                            debug!("poll {} {:?}", fd, pevent);
                            buffer[num] = ev.clone();
                            num += 1;
                        }
                    }
                }
            }
            if current_nsec() >= end || num > 0 {
                break num;
            }
        };

        Ok(n)
    }

    pub async fn sys_copy_file_range(
        &self,
        fd_in: usize,
        off_in: UserRef<usize>,
        fd_out: usize,
        off_out: UserRef<usize>,
        len: usize,
        flags: usize,
    ) -> SysResult {
        assert_eq!(flags, 0);
        debug!(
            "sys_copy_file_range @ fd_in: {}, off_in: {}, fd_out: {}, off_out: {}, len: {}",
            fd_in, off_in, fd_out, off_out, len
        );
        let in_file = self.task.get_fd(fd_in).ok_or(Errno::EBADF)?;
        let out_file = self.task.get_fd(fd_out).ok_or(Errno::EBADF)?;
        let mut buffer = vec![0u8; len];
        let rsize = if off_in.is_valid() {
            let rsize = in_file.readat(*off_in.get_ref(), &mut buffer)?;
            *off_in.get_mut() += rsize;
            rsize
        } else {
            in_file.read(&mut buffer)?
        };

        if rsize == 0 {
            return Ok(0);
        }

        if off_out.is_valid() {
            *off_out.get_mut() += out_file.writeat(*off_out.get_ref(), &mut buffer[..rsize])?;
        } else {
            out_file.write(&buffer[..rsize])?;
        }

        Ok(rsize)
    }
}
