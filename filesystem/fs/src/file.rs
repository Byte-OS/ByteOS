use crate::{dentry::get_mounted, pathbuf::PathBuf, WaitBlockingRead, WaitBlockingWrite};
use alloc::{string::String, sync::Arc, vec::Vec};
use libc_types::{
    fcntl::OpenFlags,
    poll::PollEvent,
    types::{Dirent64, Stat, StatFS, TimeSpec},
};
use sync::Mutex;
use syscalls::Errno;
use vfscore::{DirEntry, FileType, INodeInterface, SeekFrom, VfsResult};

pub struct File {
    pub inner: Arc<dyn INodeInterface>,
    path_buf: PathBuf,
    pub offset: Mutex<usize>,
    pub flags: Mutex<OpenFlags>,
}

impl File {
    pub fn open<T: Into<PathBuf>>(path: T, flags: OpenFlags) -> VfsResult<File> {
        let path_buf = path.into();
        let (de, path) = get_mounted(&path_buf);
        let mut file = de.node().clone();

        if path.levels() > 0 {
            for name in path.dir().iter() {
                file = file.lookup(name)?;
            }

            if flags.contains(OpenFlags::CREAT) {
                file.create(
                    &path.filename(),
                    if flags.contains(OpenFlags::DIRECTORY) {
                        FileType::Directory
                    } else {
                        FileType::File
                    },
                )?;
            }
            file = file.lookup(&path.filename())?;
        }

        Ok(Self {
            inner: file,
            path_buf,
            offset: Mutex::new(0),
            flags: Mutex::new(flags),
        })
    }

    pub fn new_dev(inner: Arc<dyn INodeInterface>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            offset: Mutex::new(0),
            path_buf: PathBuf::empty(),
            flags: Mutex::new(OpenFlags::RDWR),
        })
    }

    pub fn remove_self(&self) -> VfsResult<()> {
        let dir = Self::open(self.path_buf.dir(), OpenFlags::DIRECTORY)?;
        dir.remove(&self.path_buf.filename())
    }

    pub fn get_bare_file(&self) -> Arc<dyn INodeInterface> {
        self.inner.clone()
    }

    #[inline(always)]
    fn check_writeable(&self) -> Result<(), Errno> {
        let flags = self.flags.lock().clone();
        if flags.contains(OpenFlags::RDWR) | flags.contains(OpenFlags::WRONLY) {
            Ok(())
        } else {
            Err(Errno::EPERM)
        }
    }

    #[inline]
    pub fn path(&self) -> String {
        self.path_buf.path()
    }

    pub fn path_buf(&self) -> PathBuf {
        self.path_buf.clone()
    }

    pub fn file_size(&self) -> VfsResult<usize> {
        let mut stat = Stat::default();
        self.inner.stat(&mut stat)?;
        Ok(stat.size as _)
    }

    pub fn file_type(&self) -> VfsResult<FileType> {
        let mut stat = Stat::default();
        self.inner.stat(&mut stat)?;
        Ok(stat.mode.into())
    }

    pub fn getdents(&self, buffer: &mut [u8]) -> Result<usize, Errno> {
        let buf_ptr = buffer.as_mut_ptr() as usize;
        let len = buffer.len();
        let mut ptr: usize = buf_ptr;
        let mut finished = 0;
        for (i, x) in self
            .read_dir()?
            .iter()
            .enumerate()
            .skip(*self.offset.lock())
        {
            let filename = &x.filename;
            let file_bytes = filename.as_bytes();
            let current_len = size_of::<Dirent64>() + file_bytes.len() + 1;
            if len - (ptr - buf_ptr) < current_len {
                break;
            }

            // let dirent = c2rust_ref(ptr as *mut Dirent);
            let dirent: &mut Dirent64 = unsafe { (ptr as *mut Dirent64).as_mut() }.unwrap();

            dirent.ino = 0;
            dirent.off = current_len as i64;
            dirent.reclen = current_len as u16;

            dirent.ftype = 0; // 0 ftype is file

            let buffer = unsafe {
                core::slice::from_raw_parts_mut(dirent.name.as_mut_ptr(), file_bytes.len() + 1)
            };
            buffer[..file_bytes.len()].copy_from_slice(file_bytes);
            buffer[file_bytes.len()] = b'\0';
            ptr += current_len;
            finished = i + 1;
        }
        *self.offset.lock() = finished;
        Ok(ptr - buf_ptr)
    }
}

impl File {
    pub fn mkdir(&self, name: &str) -> Result<(), Errno> {
        self.inner.mkdir(name)
    }

    pub fn rmdir(&self, name: &str) -> Result<(), Errno> {
        self.inner.rmdir(name)
    }

    pub fn remove(&self, name: &str) -> Result<(), Errno> {
        self.inner.remove(name)
    }

    pub fn moveto(&self, _path: &str) -> Result<Self, Errno> {
        todo!("Move the file? to other location")
    }

    pub fn read_dir(&self) -> Result<Vec<DirEntry>, Errno> {
        self.inner.read_dir()
    }

    pub fn lookup(&self, name: &str) -> Result<Arc<dyn INodeInterface>, Errno> {
        self.inner.lookup(name)
    }

    pub fn ioctl(&self, command: usize, arg: usize) -> Result<usize, Errno> {
        self.inner.ioctl(command, arg)
    }

    pub fn truncate(&self, size: usize) -> Result<(), Errno> {
        self.inner.truncate(size)
    }

    pub fn flush(&self) -> Result<(), Errno> {
        self.inner.flush()
    }

    pub fn resolve_link(&self) -> Result<String, Errno> {
        self.inner.resolve_link()
    }

    pub fn link(&self, name: &str, src: Arc<dyn INodeInterface>) -> Result<(), Errno> {
        self.inner.link(name, src)
    }

    pub fn unlink(&self, name: &str) -> Result<(), Errno> {
        self.inner.unlink(name)
    }

    pub fn stat(&self, stat: &mut Stat) -> Result<(), Errno> {
        self.inner.stat(stat)?;
        stat.dev = 0;
        Ok(())
    }

    #[inline]
    pub fn symlink(&self, name: &str, target: &str) -> Result<(), Errno> {
        self.inner.symlink(name, target)
    }

    pub fn mount(&self, path: &str) -> Result<(), Errno> {
        self.inner.mount(path)
    }

    pub fn umount(&self) -> Result<(), Errno> {
        self.inner.umount()
    }

    pub fn statfs(&self, statfs: &mut StatFS) -> Result<(), Errno> {
        self.inner.statfs(statfs)
    }

    pub fn utimes(&self, times: &mut [TimeSpec]) -> Result<(), Errno> {
        self.inner.utimes(times)
    }

    pub fn poll(&self, events: PollEvent) -> Result<PollEvent, Errno> {
        self.inner.poll(events)
    }
}

impl File {
    pub fn readat(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, Errno> {
        self.inner.readat(offset, buffer)
    }

    pub fn writeat(&self, offset: usize, buffer: &[u8]) -> Result<usize, Errno> {
        self.check_writeable()?;
        if buffer.is_empty() {
            return Ok(0);
        }
        self.inner.writeat(offset, buffer)
    }

    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, Errno> {
        let offset = *self.offset.lock();
        self.inner
            .readat(offset, buffer)
            .inspect(|x| *self.offset.lock() += x)
    }

    pub fn write(&self, buffer: &[u8]) -> Result<usize, Errno> {
        self.check_writeable()?;
        if buffer.is_empty() {
            return Ok(0);
        }
        let offset = *self.offset.lock();
        self.inner
            .writeat(offset, buffer)
            .inspect(|x| *self.offset.lock() += x)
    }

    pub async fn async_read(&self, buffer: &mut [u8]) -> Result<usize, Errno> {
        let offset = *self.offset.lock();
        if self.flags.lock().contains(OpenFlags::NONBLOCK) {
            self.inner.readat(offset, buffer)
        } else {
            WaitBlockingRead(self.inner.clone(), buffer, offset).await
        }
        .inspect(|x| *self.offset.lock() += x)
    }

    pub async fn async_write(&self, buffer: &[u8]) -> Result<usize, Errno> {
        // self.check_writeable()?;
        if buffer.is_empty() {
            return Ok(0);
        }
        let offset = *self.offset.lock();
        WaitBlockingWrite(self.inner.clone(), buffer, offset)
            .await
            .inspect(|x| *self.offset.lock() += x)
    }

    pub fn seek(&self, seek_from: SeekFrom) -> Result<usize, Errno> {
        let offset = *self.offset.lock();
        let mut stat = Stat::default();
        self.inner.stat(&mut stat)?;
        let mut new_off = match seek_from {
            SeekFrom::SET(off) => off as isize,
            SeekFrom::CURRENT(off) => offset as isize + off,
            SeekFrom::END(off) => stat.size as isize + off,
        };
        if new_off < 0 {
            new_off = 0;
        }
        // assert!(new_off >= 0);
        *self.offset.lock() = new_off as _;
        Ok(new_off as _)
    }
}
