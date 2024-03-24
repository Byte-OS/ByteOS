use core::{
    mem::size_of,
    ops::{Deref, DerefMut},
};

use alloc::{string::String, sync::Arc, vec::Vec};
use fs::{
    dentry::{self, dentry_open, dentry_root, DentryNode},
    INodeInterface, VfsError, WaitBlockingRead, WaitBlockingWrite,
};
use sync::Mutex;
use vfscore::{
    DirEntry, Dirent64, Metadata, OpenFlags, PollEvent, SeekFrom, Stat, StatFS, TimeSpec,
};

const FILE_MAX: usize = 255;
const FD_NONE: Option<Arc<FileItem>> = Option::None;

#[derive(Clone)]
pub struct FileTable(pub Vec<Option<Arc<FileItem>>>);

impl FileTable {
    pub fn new() -> Self {
        let mut file_table: Vec<Option<Arc<FileItem>>> = vec![FD_NONE; FILE_MAX];
        file_table[..3].fill(Some(
            FileItem::fs_open("/dev/ttyv0", OpenFlags::O_RDWR).expect("can't read tty file"),
        ));
        Self(file_table)
    }
}

impl Deref for FileTable {
    type Target = Vec<Option<Arc<FileItem>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FileTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub fn rlimits_new() -> Vec<usize> {
    let mut rlimits = vec![0usize; 8];
    rlimits[7] = FILE_MAX;
    rlimits
}

bitflags! {
    #[derive(Clone, Debug)]
    pub struct FileOptions: u8 {
        /// Hangup.
        const R = 1;
        const W = 1 << 1;
        const X = 1 << 3;
        /// Create file.
        const C = 1 << 4;
    }
}

impl Default for FileOptions {
    fn default() -> Self {
        FileOptions::R | FileOptions::W | FileOptions::X
    }
}

pub struct FileItem {
    pub inner: Arc<dyn INodeInterface>,
    pub dentry: Option<Arc<DentryNode>>,
    pub options: FileOptions,
    pub offset: Mutex<usize>,
    pub flags: Mutex<OpenFlags>,
}

impl<'a> FileItem {
    pub fn new(
        inner: Arc<dyn INodeInterface>,
        dentry: Option<Arc<DentryNode>>,
        options: FileOptions,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner,
            options,
            dentry,
            offset: Mutex::new(0),
            flags: Mutex::new(OpenFlags::NONE),
        })
    }

    /// Get root directory FileItem.
    pub fn root() -> Arc<Self> {
        let dentry = dentry_root();
        Self::new(
            dentry.node.clone(),
            Some(dentry),
            FileOptions::R | FileOptions::W,
        )
    }

    pub fn new_dev(inner: Arc<dyn INodeInterface>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            offset: Mutex::new(0),
            dentry: None,
            options: FileOptions::default(),
            flags: Mutex::new(OpenFlags::NONE),
        })
    }

    pub fn get_bare_file(&self) -> Arc<dyn INodeInterface> {
        self.inner.clone()
    }

    pub fn fs_open(path: &str, open_flags: OpenFlags) -> Result<Arc<Self>, VfsError> {
        let mut options = FileOptions::R | FileOptions::X;
        if open_flags.contains(OpenFlags::O_WRONLY)
            || open_flags.contains(OpenFlags::O_RDWR)
            || open_flags.contains(OpenFlags::O_ACCMODE)
        {
            options = options.union(FileOptions::W);
        }
        let dentry_node = dentry::dentry_open(dentry_root(), path, OpenFlags::NONE)?;
        let offset = if open_flags.contains(OpenFlags::O_APPEND) {
            dentry_node.node.metadata()?.size
        } else {
            0
        };
        Ok(Arc::new(Self {
            inner: dentry_node.node.clone(),
            options,
            dentry: Some(dentry_node),
            offset: Mutex::new(offset),
            flags: Mutex::new(open_flags),
        }))
    }

    pub fn dentry_open(&self, path: &str, flags: OpenFlags) -> Result<Arc<Self>, VfsError> {
        let mut options = FileOptions::R | FileOptions::X;
        if flags.contains(OpenFlags::O_WRONLY)
            || flags.contains(OpenFlags::O_RDWR)
            || flags.contains(OpenFlags::O_ACCMODE)
        {
            options = options.union(FileOptions::W);
        }
        assert!(self.dentry.is_some());
        dentry_open(self.dentry.clone().unwrap(), path, flags.clone()).map(|x| {
            Arc::new(FileItem {
                inner: x.node.clone(),
                dentry: Some(x),
                offset: Mutex::new(0),
                flags: Mutex::new(flags.clone()),
                options,
            })
        })
    }

    #[inline(always)]
    fn check_writeable(&self) -> Result<(), VfsError> {
        if self.options.contains(FileOptions::W) {
            Ok(())
        } else {
            Err(VfsError::NotWriteable)
        }
    }

    pub fn path(&self) -> Result<String, VfsError> {
        // Ok(&self.path)
        // dentry_open(dentry, path, flags)
        match &self.dentry {
            Some(dentry) => Ok(dentry.path()),
            None => Err(VfsError::NotFile),
        }
    }

    pub fn getdents(&self, buffer: &mut [u8]) -> Result<usize, VfsError> {
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

impl FileItem {
    pub fn mkdir(&self, name: &str) -> Result<Arc<dyn INodeInterface>, VfsError> {
        self.inner.mkdir(name)
    }

    pub fn rmdir(&self, name: &str) -> Result<(), VfsError> {
        self.inner.rmdir(name)
    }

    pub fn remove(&self, name: &str) -> Result<(), VfsError> {
        self.inner.remove(name)
    }

    pub fn moveto(&self, _path: &str) -> Result<Self, VfsError> {
        todo!("Move the file? to other location")
    }

    pub fn remove_self(&self) -> Result<(), VfsError> {
        match &self.dentry {
            Some(dentry) => {
                let filename = &dentry.filename;
                if let Some(parent) = dentry.parent.upgrade() {
                    parent.node.remove(filename)?;
                    parent.children.lock().retain(|x| &x.filename != filename);
                }
                Ok(())
            }
            None => Err(VfsError::FileNotFound),
        }
    }

    pub fn touch(&self, name: &str) -> Result<Arc<dyn INodeInterface>, VfsError> {
        self.inner.touch(name)
    }

    pub fn read_dir(&self) -> Result<Vec<DirEntry>, VfsError> {
        self.inner.read_dir()
    }

    pub fn metadata(&self) -> Result<Metadata, VfsError> {
        self.inner.metadata()
    }

    pub fn lookup(&self, name: &str) -> Result<Arc<dyn INodeInterface>, VfsError> {
        self.inner.lookup(name)
    }

    pub fn open(&self, name: &str, flags: OpenFlags) -> Result<Arc<dyn INodeInterface>, VfsError> {
        self.inner.open(name, flags)
    }

    pub fn ioctl(&self, command: usize, arg: usize) -> Result<usize, VfsError> {
        self.inner.ioctl(command, arg)
    }

    pub fn truncate(&self, size: usize) -> Result<(), VfsError> {
        // self.check_writeable()?;
        // let mut offset = self.offset.lock();
        // if *offset > size {
        //     *offset = size;
        // }
        self.inner.truncate(size)
    }

    pub fn flush(&self) -> Result<(), VfsError> {
        self.inner.flush()
    }

    pub fn resolve_link(&self) -> Result<String, VfsError> {
        self.inner.resolve_link()
    }

    pub fn link(&self, name: &str, src: Arc<dyn INodeInterface>) -> Result<(), VfsError> {
        self.inner.link(name, src)
    }

    pub fn unlink(&self, name: &str) -> Result<(), VfsError> {
        self.inner.unlink(name)
    }

    pub fn stat(&self, stat: &mut Stat) -> Result<(), VfsError> {
        self.inner.stat(stat)?;
        stat.dev = 0;
        Ok(())
    }

    pub fn mount(&self, path: &str) -> Result<(), VfsError> {
        self.inner.mount(path)
    }

    pub fn umount(&self) -> Result<(), VfsError> {
        self.inner.umount()
    }

    pub fn statfs(&self, statfs: &mut StatFS) -> Result<(), VfsError> {
        self.inner.statfs(statfs)
    }

    pub fn utimes(&self, times: &mut [TimeSpec]) -> Result<(), VfsError> {
        self.inner.utimes(times)
    }

    pub fn poll(&self, events: PollEvent) -> Result<PollEvent, VfsError> {
        self.inner.poll(events)
    }
}

impl FileItem {
    pub fn readat(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, VfsError> {
        self.inner.readat(offset, buffer)
    }

    pub fn writeat(&self, offset: usize, buffer: &[u8]) -> Result<usize, VfsError> {
        self.check_writeable()?;
        if buffer.len() == 0 {
            return Ok(0);
        }
        self.inner.writeat(offset, buffer)
    }

    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let offset = *self.offset.lock();
        self.inner.readat(offset, buffer).map(|x| {
            *self.offset.lock() += x;
            x
        })
    }

    pub fn write(&self, buffer: &[u8]) -> Result<usize, VfsError> {
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

    pub async fn async_read(&self, buffer: &mut [u8]) -> Result<usize, VfsError> {
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

    pub async fn async_write(&self, buffer: &[u8]) -> Result<usize, VfsError> {
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

    pub fn seek(&self, seek_from: SeekFrom) -> Result<usize, VfsError> {
        let offset = *self.offset.lock();
        let mut new_off = match seek_from {
            SeekFrom::SET(off) => off as isize,
            SeekFrom::CURRENT(off) => offset as isize + off,
            SeekFrom::END(off) => self.metadata()?.size as isize + off,
        };
        if new_off < 0 {
            new_off = 0;
        }
        // assert!(new_off >= 0);
        *self.offset.lock() = new_off as _;
        Ok(new_off as _)
    }
}
