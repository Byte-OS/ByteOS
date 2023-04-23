#![no_std]
#![feature(drain_filter)]

extern crate alloc;

use core::cmp::{self, min};

use alloc::{format, string::String, sync::Arc, vec::Vec};
use sync::Mutex;
use vfscore::{
    DirEntry, FileSystem, FileType, INodeInterface, Metadata, MountedInfo, SeekFrom, Stat,
    TimeSpec, VfsError, VfsResult, UTIME_OMIT,
};

pub struct RamFs {
    root: Arc<RamDirInner>,
}

impl RamFs {
    pub fn new() -> Arc<Self> {
        let inner = Arc::new(RamDirInner {
            name: String::from(""),
            children: Mutex::new(Vec::new()),
            dir_path: String::from(""),
        });
        Arc::new(Self { root: inner })
    }
}

impl FileSystem for RamFs {
    fn root_dir(&'static self, mi: MountedInfo) -> Arc<dyn INodeInterface> {
        Arc::new(RamDir {
            inner: self.root.clone(),
            mi,
        })
    }

    fn name(&self) -> &str {
        "ramfs"
    }
}

pub struct RamDirInner {
    name: String,
    children: Mutex<Vec<FileContainer>>,
    dir_path: String,
}

// TODO: use frame insteads of Vec.
pub struct RamFileInner {
    name: String,
    content: Mutex<Vec<u8>>,
    dir_path: String,
    times: Mutex<[TimeSpec; 3]>, // ctime, atime, mtime.
}

pub enum FileContainer {
    File(Arc<RamFileInner>),
    Dir(Arc<RamDirInner>),
}

impl FileContainer {
    #[inline]
    fn to_inode(&self, mi: MountedInfo) -> Arc<dyn INodeInterface> {
        match self {
            FileContainer::File(file) => Arc::new(RamFile {
                offset: Mutex::new(0),
                inner: file.clone(),
                mi,
            }),
            FileContainer::Dir(dir) => Arc::new(RamDir {
                inner: dir.clone(),
                mi,
            }),
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
    mi: MountedInfo,
}

impl INodeInterface for RamDir {
    fn open(&self, name: &str, _flags: vfscore::OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map(|x| x.to_inode(self.mi.clone()))
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
            dir_path: format!("{}/{}", self.inner.dir_path, self.inner.name),
            times: Mutex::new([Default::default(); 3]),
        });

        let new_file = Arc::new(RamFile {
            offset: Mutex::new(0),
            inner: new_inner.clone(),
            mi: self.mi.clone(),
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
            dir_path: format!("{}/{}", self.inner.dir_path, self.inner.name),
        });

        let new_dir = Arc::new(RamDir {
            inner: new_inner.clone(),
            mi: self.mi.clone(),
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

    fn unlink(&self, name: &str) -> VfsResult<()> {
        self.remove(name)
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

    fn path(&self) -> VfsResult<String> {
        Ok(format!(
            "{}/{}/{}",
            self.mi.path, self.inner.dir_path, self.inner.name
        ))
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.dev = self.mi.fs_id as u64;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = 0; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 1000;
        stat.gid = 1000;
        stat.size = 0;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        stat.mtime = Default::default();
        stat.atime = Default::default();
        Ok(())
    }
}

pub struct RamFile {
    offset: Mutex<usize>,
    inner: Arc<RamFileInner>,
    mi: MountedInfo,
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
        let wsize = buffer.len();

        let part1 = cmp::min(file_size - offset, wsize);
        let mut content = self.inner.content.lock();
        content[offset..offset + part1].copy_from_slice(&buffer[..part1]);
        // extend content if offset + buffer > content.len()
        content.extend_from_slice(&buffer[part1..]);

        *self.offset.lock() += wsize;
        Ok(wsize)
    }

    fn seek(&self, seek: SeekFrom) -> VfsResult<usize> {
        let new_off = match seek {
            SeekFrom::SET(off) => off as isize,
            SeekFrom::CURRENT(off) => *self.offset.lock() as isize + off,
            SeekFrom::END(off) => self.inner.content.lock().len() as isize + off,
        };
        match new_off >= 0 {
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

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        Ok(Metadata {
            filename: &self.inner.name,
            inode: 0,
            file_type: FileType::File,
            size: self.inner.content.lock().len(),
            childrens: 0,
        })
    }

    fn path(&self) -> VfsResult<String> {
        Ok(format!(
            "{}/{}/{}",
            self.mi.path, self.inner.dir_path, self.inner.name
        ))
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.dev = self.mi.fs_id as u64;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = 0; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 1000;
        stat.gid = 1000;
        stat.size = self.inner.content.lock().len() as u64;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id

        stat.atime = self.inner.times.lock()[1];
        stat.mtime = self.inner.times.lock()[2];
        Ok(())
    }

    fn utimes(&self, times: &mut [vfscore::TimeSpec]) -> VfsResult<()> {
        if times[0].nsec != UTIME_OMIT {
            self.inner.times.lock()[1] = times[0];
        }
        if times[1].nsec != UTIME_OMIT {
            self.inner.times.lock()[2] = times[1];
        }
        Ok(())
    }
}
