use core::ops::{Deref, DerefMut};

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use devfs::Tty;
use fs::{mount::open, INodeInterface, VfsError, WaitBlockingRead, WaitBlockingWrite};
use vfscore::{DirEntry, MMapFlags, Metadata, OpenFlags, PollEvent, Stat, StatFS, TimeSpec};

const FILE_MAX: usize = 255;
const FD_NONE: Option<FileItem> = Option::None;

#[derive(Clone)]
// pub struct FileTable(pub Vec<Option<File>>);
pub struct FileTable(pub Vec<Option<FileItem>>);

impl FileTable {
    pub fn new() -> Self {
        let mut file_table: Vec<Option<FileItem>> = vec![FD_NONE; FILE_MAX];
        file_table[..3].fill(Some(FileItem::new(
            Arc::new(Tty::new()),
            Default::default(),
        )));
        Self(file_table)
    }
}

impl Deref for FileTable {
    type Target = Vec<Option<FileItem>>;

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
    }
}

impl Default for FileOptions {
    fn default() -> Self {
        FileOptions::R | FileOptions::W | FileOptions::X
    }
}

#[derive(Clone)]
pub struct FileItem {
    pub path: String,
    pub inner: Arc<dyn INodeInterface>,
    pub options: FileOptions,
    pub flags: OpenFlags,
}

pub trait FileItemInterface: INodeInterface {
    async fn async_read(&self, buffer: &mut [u8]) -> Result<usize, VfsError>;
    async fn async_write(&self, buffer: &[u8]) -> Result<usize, VfsError>;
}

impl<'a> FileItem {
    pub fn new(inner: Arc<dyn INodeInterface>, options: FileOptions) -> Self {
        Self {
            path: String::new(),
            inner,
            options,
            flags: OpenFlags::NONE,
        }
    }

    pub fn get_bare_file(&self) -> Arc<dyn INodeInterface> {
        self.inner.clone()
    }

    pub fn fs_open(path: &str, options: FileOptions) -> Result<Self, VfsError> {
        Ok(Self {
            path: path.to_string(),
            inner: open(path)?,
            options,
            flags: OpenFlags::NONE,
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

    pub fn path(&'a self) ->Result< &'a str, VfsError> {
        Ok(&self.path)
    }
}

#[allow(dead_code)]
impl INodeInterface for FileItem {
    fn seek(&self, offset: fs::SeekFrom) -> Result<usize, VfsError> {
        self.inner.seek(offset)
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, VfsError> {
        self.inner.read(buffer)
    }

    fn write(&self, buffer: &[u8]) -> Result<usize, VfsError> {
        self.check_writeable()?;
        self.inner.write(buffer)
    }

    fn mkdir(&self, name: &str) -> Result<Arc<dyn INodeInterface>, VfsError> {
        self.inner.mkdir(name)
    }

    fn rmdir(&self, name: &str) -> Result<(), VfsError> {
        self.inner.rmdir(name)
    }

    fn remove(&self, name: &str) -> Result<(), VfsError> {
        self.inner.remove(name)
    }

    fn touch(&self, name: &str) -> Result<Arc<dyn INodeInterface>, VfsError> {
        self.inner.touch(name)
    }

    fn read_dir(&self) -> Result<Vec<DirEntry>, VfsError> {
        self.inner.read_dir()
    }

    fn metadata(&self) -> Result<Metadata, VfsError> {
        self.inner.metadata()
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn INodeInterface>, VfsError> {
        self.inner.lookup(name)
    }

    fn open(&self, name: &str, flags: OpenFlags) -> Result<Arc<dyn INodeInterface>, VfsError> {
        self.inner.open(name, flags)
    }

    fn ioctl(&self, command: usize, arg: usize) -> Result<usize, VfsError> {
        self.inner.ioctl(command, arg)
    }

    fn truncate(&self, size: usize) -> Result<(), VfsError> {
        // self.check_writeable()?;
        self.inner.truncate(size)
    }

    fn flush(&self) -> Result<(), VfsError> {
        self.inner.flush()
    }

    fn resolve_link(&self) -> Result<String, VfsError> {
        self.inner.resolve_link()
    }

    fn link(&self, name: &str, src: Arc<dyn INodeInterface>) -> Result<(), VfsError> {
        self.inner.link(name, src)
    }

    fn unlink(&self, name: &str) -> Result<(), VfsError> {
        self.inner.unlink(name)
    }

    fn mmap(&self, offset: usize, size: usize, flags: MMapFlags) -> Result<usize, VfsError> {
        self.inner.mmap(offset, size, flags)
    }

    fn stat(&self, stat: &mut Stat) -> Result<(), VfsError> {
        self.inner.stat(stat)?;
        stat.dev = 0;
        Ok(())
    }

    fn mount(&self, path: &str) -> Result<(), VfsError> {
        self.inner.mount(path)
    }

    fn umount(&self) -> Result<(), VfsError> {
        self.inner.umount()
    }

    fn statfs(&self, statfs: &mut StatFS) -> Result<(), VfsError> {
        self.inner.statfs(statfs)
    }

    fn getdents(&self, buffer: &mut [u8]) -> Result<usize, VfsError> {
        self.inner.getdents(buffer)
    }

    fn utimes(&self, times: &mut [TimeSpec]) -> Result<(), VfsError> {
        self.inner.utimes(times)
    }

    fn poll(&self, events: PollEvent) -> Result<PollEvent, VfsError> {
        self.inner.poll(events)
    }
}

impl FileItemInterface for FileItem {
    async fn async_read(&self, buffer: &mut [u8]) -> Result<usize, VfsError> {
        if self.flags.contains(OpenFlags::O_NONBLOCK) {
            self.read(buffer)
        } else {
            WaitBlockingRead(self.inner.clone(), buffer).await
        }
    }

    async fn async_write(&self, buffer: &[u8]) -> Result<usize, VfsError> {
        // self.check_writeable()?;
        WaitBlockingWrite(self.inner.clone(), &buffer).await
    }
}
