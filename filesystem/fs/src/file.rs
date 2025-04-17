use crate::{dentry::get_mounted, WaitBlockingRead, WaitBlockingWrite};
use alloc::{borrow::ToOwned, string::String, sync::Arc, vec::Vec};
use sync::Mutex;
use syscalls::Errno;
use vfscore::{
    DirEntry, Dirent64, FileType, INodeInterface, OpenFlags, PollEvent, SeekFrom, Stat, StatFS,
    StatMode, TimeSpec, VfsResult,
};

pub struct File {
    pub inner: Arc<dyn INodeInterface>,
    path: String,
    pub offset: Mutex<usize>,
    pub flags: Mutex<OpenFlags>,
}

impl<'a> File {
    pub fn open(path: &str, flags: OpenFlags) -> VfsResult<File> {
        let fullpath = if path.starts_with("..") {
            path.trim_start_matches("..").to_owned()
        } else if path.starts_with(".") {
            path.trim_start_matches(".").to_owned()
        } else if path.starts_with("/") {
            path.to_owned()
        } else {
            String::from("/") + path
        };
        log::warn!("original path: {}", path);
        let (de, path) = get_mounted(fullpath.clone());
        log::warn!("mounted path: {} name: {}", path, de.fs.name());
        let mut file = de.node().clone();

        let paths = path.split("/").into_iter().filter(|x| *x != "");
        let len = paths.clone().count();
        if len > 0 {
            let filename = paths.clone().last().unwrap();
            for x in paths.take(len - 1) {
                file = match x {
                    "." | "" => file,
                    filename => file.lookup(filename)?,
                }
            }
            if flags.contains(OpenFlags::O_CREAT) {
                file.create(
                    filename,
                    if flags.contains(OpenFlags::O_DIRECTORY) {
                        FileType::Directory
                    } else {
                        FileType::File
                    },
                )?;
            }
            file = file.lookup(filename)?;
        }

        Ok(Self {
            inner: file,
            path: fullpath,
            offset: Mutex::new(0),
            flags: Mutex::new(flags),
        })
    }

    pub fn new_dev(inner: Arc<dyn INodeInterface>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            offset: Mutex::new(0),
            path: String::new(),
            flags: Mutex::new(OpenFlags::O_RDWR),
        })
    }

    pub fn remove_self(&self) -> VfsResult<()> {
        // TODO: 更加细化 remove self
        assert!(!self.path.ends_with("/"));
        let (dir, name) = self.path.rsplit_once("/").unwrap();
        let dir = Self::open(dir, OpenFlags::O_DIRECTORY)?;
        dir.remove(name)
    }

    pub fn get_bare_file(&self) -> Arc<dyn INodeInterface> {
        self.inner.clone()
    }

    #[inline(always)]
    fn check_writeable(&self) -> Result<(), Errno> {
        let flags = self.flags.lock().clone();
        if flags.contains(OpenFlags::O_RDWR) | flags.contains(OpenFlags::O_WRONLY) {
            Ok(())
        } else {
            Err(Errno::EPERM)
        }
    }

    #[inline]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn file_size(&self) -> VfsResult<usize> {
        let mut stat = Stat::default();
        self.inner.stat(&mut stat)?;
        Ok(stat.size as _)
    }

    pub fn file_type(&self) -> VfsResult<FileType> {
        let mut stat = Stat::default();
        self.inner.stat(&mut stat)?;
        if stat.mode.contains(StatMode::SOCKET) {
            Ok(FileType::Socket)
        } else if stat.mode.contains(StatMode::DIR) {
            Ok(FileType::Directory)
        } else {
            Ok(FileType::File)
        }
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
            ptr = ptr + current_len;
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
        if buffer.len() == 0 {
            return Ok(0);
        }
        self.inner.writeat(offset, buffer)
    }

    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, Errno> {
        let offset = *self.offset.lock();
        self.inner.readat(offset, buffer).map(|x| {
            *self.offset.lock() += x;
            x
        })
    }

    pub fn write(&self, buffer: &[u8]) -> Result<usize, Errno> {
        self.check_writeable()?;
        if buffer.len() == 0 {
            return Ok(0);
        }
        let offset = *self.offset.lock();
        self.inner.writeat(offset, buffer).map(|x| {
            *self.offset.lock() += x;
            x
        })
    }

    pub async fn async_read(&self, buffer: &mut [u8]) -> Result<usize, Errno> {
        let offset = *self.offset.lock();
        if self.flags.lock().contains(OpenFlags::O_NONBLOCK) {
            self.inner.readat(offset, buffer)
        } else {
            WaitBlockingRead(self.inner.clone(), buffer, offset).await
        }
        .map(|x| {
            *self.offset.lock() += x;
            x
        })
    }

    pub async fn async_write(&self, buffer: &[u8]) -> Result<usize, Errno> {
        // self.check_writeable()?;
        if buffer.len() == 0 {
            return Ok(0);
        }
        let offset = *self.offset.lock();
        WaitBlockingWrite(self.inner.clone(), &buffer, offset)
            .await
            .map(|x| {
                *self.offset.lock() += x;
                x
            })
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
