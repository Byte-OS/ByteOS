#![no_std]
#![feature(drain_filter)]

extern crate alloc;

use core::cmp::min;

use alloc::{string::String, sync::Arc, vec::Vec};
use sync::Mutex;
use vfscore::{
    DirEntry, FileSystem, FileType, INodeInterface, Metadata, MountedInfo, SeekFrom, VfsError,
    VfsResult,
};

pub struct RamFs {
    root: Arc<RamDirInner>,
}

impl RamFs {
    pub fn new() -> Arc<Self> {
        let inner = Arc::new(RamDirInner {
            name: String::from(""),
            children: Mutex::new(Vec::new()),
        });
        Arc::new(Self { root: inner })
    }
}

impl FileSystem for RamFs {
    fn root_dir(&'static self, _mi: MountedInfo) -> Arc<dyn INodeInterface> {
        Arc::new(RamDir {
            inner: self.root.clone(),
        })
    }

    fn name(&self) -> &str {
        "ramfs"
    }
}

pub struct RamDirInner {
    name: String,
    children: Mutex<Vec<FileContainer>>,
}

// TODO: use frame insteads of Vec.
pub struct RamFileInner {
    name: String,
    content: Mutex<Vec<u8>>,
}

pub enum FileContainer {
    File(Arc<RamFileInner>),
    Dir(Arc<RamDirInner>),
}

impl FileContainer {
    #[inline]
    fn to_inode(&self) -> Arc<dyn INodeInterface> {
        match self {
            FileContainer::File(file) => Arc::new(RamFile {
                offset: Mutex::new(0),
                inner: file.clone(),
            }),
            FileContainer::Dir(dir) => Arc::new(RamDir { inner: dir.clone() }),
        }
    }

    #[inline]
    fn filename(&self) -> &str {
        match self {
            FileContainer::File(file) => &file.name,
            FileContainer::Dir(dir) => &dir.name,
        }
    }
}

pub struct RamDir {
    inner: Arc<RamDirInner>,
}

impl INodeInterface for RamDir {
    fn open(&self, name: &str, _flags: vfscore::OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map(|x| x.to_inode())
            .ok_or(VfsError::FileNotFound)
    }

    fn touch(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(VfsError::AlreadyExists))?;

        let new_inner = Arc::new(RamFileInner {
            name: String::from(name),
            content: Mutex::new(Vec::new()),
        });

        let new_file = Arc::new(RamFile {
            offset: Mutex::new(0),
            inner: new_inner.clone(),
        });

        self.inner
            .children
            .lock()
            .push(FileContainer::File(new_inner));

        Ok(new_file)
    }

    fn mkdir(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(VfsError::AlreadyExists))?;

        let new_inner = Arc::new(RamDirInner {
            name: String::from(name),
            children: Mutex::new(Vec::new()),
        });

        let new_dir = Arc::new(RamDir {
            inner: new_inner.clone(),
        });

        self.inner
            .children
            .lock()
            .push(FileContainer::Dir(new_inner));

        Ok(new_dir)
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        // TODO: identify whether the dir is empty(through metadata.childrens)
        // return DirectoryNotEmpty if not empty.
        let len = self
            .inner
            .children
            .lock()
            .drain_filter(|x| match x {
                FileContainer::File(_) => false,
                FileContainer::Dir(x) => x.name == name,
            })
            .count();
        match len > 0 {
            true => Ok(()),
            false => Err(VfsError::FileNotFound),
        }
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Ok(self
            .inner
            .children
            .lock()
            .iter()
            .map(|x| match x {
                FileContainer::File(file) => DirEntry {
                    filename: file.name.clone(),
                    len: file.content.lock().len(),
                    file_type: FileType::File,
                },
                FileContainer::Dir(dir) => DirEntry {
                    filename: dir.name.clone(),
                    len: 0,
                    file_type: FileType::File,
                },
            })
            .collect())
    }

    fn remove(&self, name: &str) -> VfsResult<()> {
        let len = self
            .inner
            .children
            .lock()
            .drain_filter(|x| match x {
                FileContainer::File(x) => x.name == name,
                FileContainer::Dir(_) => false,
            })
            .count();
        match len > 0 {
            true => Ok(()),
            false => Err(VfsError::FileNotFound),
        }
    }

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        Ok(Metadata {
            filename: &self.inner.name,
            inode: 0,
            file_type: FileType::Directory,
            size: 0,
            childrens: self.inner.children.lock().len(),
        })
    }
}

pub struct RamFile {
    offset: Mutex<usize>,
    inner: Arc<RamFileInner>,
}

impl INodeInterface for RamFile {
    fn read(&self, buffer: &mut [u8]) -> VfsResult<usize> {
        let offset = *self.offset.lock();
        let file_size = self.inner.content.lock().len();
        match offset >= file_size {
            true => Ok(0),
            false => {
                let read_len = min(buffer.len(), file_size - offset);
                let content = self.inner.content.lock();
                buffer[..read_len].copy_from_slice(&content[offset..(offset + read_len)]);
                *self.offset.lock() += read_len;
                Ok(read_len)
            }
        }
    }

    fn write(&self, buffer: &[u8]) -> VfsResult<usize> {
        let offset = *self.offset.lock();
        let file_size = self.inner.content.lock().len();
        let write_size = buffer.len();

        let part1 = file_size - offset;
        let mut content = self.inner.content.lock();
        content[offset..].copy_from_slice(&buffer[..part1]);
        content.extend_from_slice(&buffer[part1..]);
        Ok(write_size)
    }

    fn seek(&self, seek: SeekFrom) -> VfsResult<usize> {
        let new_off = match seek {
            SeekFrom::SET(off) => (*self.offset.lock() + off) as isize,
            SeekFrom::CURRENT(off) => *self.offset.lock() as isize + off,
            SeekFrom::END(off) => self.inner.content.lock().len() as isize + off,
        };
        match new_off > 0 {
            true => {
                *self.offset.lock() = new_off as _;
                Ok(new_off as _)
            }
            false => Err(VfsError::InvalidInput),
        }
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        self.inner.content.lock().drain(size..);
        Ok(())
    }
}
