#![no_std]
#![feature(extract_if)]
#[macro_use]
extern crate alloc;

use alloc::{string::String, sync::Arc, vec::Vec};
use core::cmp::{self, min};
use core::ops::Add;
use libc_types::consts::UTIME_OMIT;
use libc_types::types::{Stat, StatMode, TimeSpec};
use polyhal::pagetable::PAGE_SIZE;
use runtime::frame::{frame_alloc, FrameTracker};
use sync::Mutex;
use syscalls::Errno;
use vfscore::{DirEntry, FileSystem, FileType, INodeInterface, VfsResult};

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
    fn root_dir(&self) -> Arc<dyn INodeInterface> {
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
    // content: Mutex<Vec<u8>>,
    len: Mutex<usize>,
    pages: Mutex<Vec<FrameTracker>>,
    times: Mutex<[TimeSpec; 3]>, // ctime, atime, mtime.
}

#[allow(dead_code)]
pub struct RamLinkInner {
    name: String,
    link_file: Arc<dyn INodeInterface>,
}

pub enum FileContainer {
    File(Arc<RamFileInner>),
    Dir(Arc<RamDirInner>),
    Link(Arc<RamLinkInner>),
}

impl FileContainer {
    #[inline]
    fn to_inode(&self) -> VfsResult<Arc<dyn INodeInterface>> {
        match self {
            FileContainer::File(file) => Ok(Arc::new(RamFile {
                inner: file.clone(),
            })),
            FileContainer::Dir(dir) => Ok(Arc::new(RamDir { inner: dir.clone() })),
            FileContainer::Link(link) => Ok(Arc::new(RamLink {
                inner: link.clone(),
                link_file: link.link_file.clone(),
            })),
        }
    }

    #[inline]
    fn filename(&self) -> &str {
        match self {
            FileContainer::File(file) => &file.name,
            FileContainer::Dir(dir) => &dir.name,
            FileContainer::Link(link) => &link.name,
        }
    }
}

#[allow(dead_code)]
pub struct RamLink {
    inner: Arc<RamLinkInner>,
    link_file: Arc<dyn INodeInterface>,
}

pub struct RamDir {
    inner: Arc<RamDirInner>,
}

impl RamDir {
    pub const fn new(inner: Arc<RamDirInner>) -> Self {
        Self { inner }
    }
}

impl INodeInterface for RamDir {
    fn mkdir(&self, name: &str) -> VfsResult<()> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(Errno::EEXIST))?;

        let new_inner = Arc::new(RamDirInner {
            name: String::from(name),
            children: Mutex::new(Vec::new()),
        });

        self.inner
            .children
            .lock()
            .push(FileContainer::Dir(new_inner));

        Ok(())
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map(|x| x.to_inode())
            .ok_or(Errno::ENOENT)?
    }

    fn create(&self, name: &str, ty: FileType) -> VfsResult<()> {
        if ty == FileType::Directory {
            let new_inner = Arc::new(RamDirInner {
                name: String::from(name),
                children: Mutex::new(Vec::new()),
            });
            self.inner
                .children
                .lock()
                .push(FileContainer::Dir(new_inner.clone()));
            Ok(())
        } else if ty == FileType::File {
            let new_inner = Arc::new(RamFileInner {
                name: String::from(name),
                // content: Mutex::new(Vec::new()),
                times: Mutex::new([Default::default(); 3]),
                len: Mutex::new(0),
                pages: Mutex::new(vec![]),
            });
            self.inner
                .children
                .lock()
                .push(FileContainer::File(new_inner.clone()));
            Ok(())
        } else {
            unimplemented!("")
        }
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        // TODO: identify whether the dir is empty(through metadata.childrens)
        // return DirectoryNotEmpty if not empty.
        let len = self
            .inner
            .children
            .lock()
            .extract_if(.., |x| match x {
                FileContainer::Dir(x) => x.name == name,
                _ => false,
            })
            .count();
        match len > 0 {
            true => Ok(()),
            false => Err(Errno::ENOENT),
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
                    // len: file.content.lock().len(),
                    len: *file.len.lock(),
                    file_type: FileType::File,
                },
                FileContainer::Dir(dir) => DirEntry {
                    filename: dir.name.clone(),
                    len: 0,
                    file_type: FileType::Directory,
                },
                FileContainer::Link(link) => DirEntry {
                    filename: link.name.clone(),
                    len: 0,
                    file_type: FileType::Link,
                },
            })
            .collect())
    }

    fn remove(&self, name: &str) -> VfsResult<()> {
        let len = self
            .inner
            .children
            .lock()
            .extract_if(.., |x| match x {
                FileContainer::File(x) => x.name == name,
                FileContainer::Dir(_) => false,
                FileContainer::Link(x) => x.name == name,
            })
            .count();
        match len > 0 {
            true => Ok(()),
            false => Err(Errno::ENOENT),
        }
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        self.remove(name)
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::DIR; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        stat.mtime = Default::default();
        stat.atime = Default::default();
        stat.ctime = Default::default();
        Ok(())
    }

    fn link(&self, name: &str, src: Arc<dyn INodeInterface>) -> VfsResult<()> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(Errno::EEXIST))?;

        let new_inner = Arc::new(RamLinkInner {
            name: String::from(name),
            link_file: src,
        });

        self.inner
            .children
            .lock()
            .push(FileContainer::Link(new_inner));

        Ok(())
    }
}

pub struct RamFile {
    inner: Arc<RamFileInner>,
}

impl RamFile {
    pub const fn new(inner: Arc<RamFileInner>) -> Self {
        Self { inner }
    }
}

impl INodeInterface for RamFile {
    fn readat(&self, mut offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        let mut buffer_off = 0;
        // let file_size = self.inner.content.lock().len();
        log::debug!("read ramfs: offset: {} len: {}", offset, buffer.len());
        let file_size = *self.inner.len.lock();
        let inner = self.inner.pages.lock();
        match offset >= file_size {
            true => Ok(0),
            false => {
                // let origin_read_len = min(buffer.len(), file_size - offset);
                // let read_len = if offset >= real_size {
                //     min(origin_read_len, real_size - offset)
                // } else {
                //     0
                // };
                let read_len = min(buffer.len(), file_size - offset);
                let mut last_len = read_len;
                // let content = self.inner.content.lock();
                // buffer[..read_len].copy_from_slice(&content[offset..(offset + read_len)]);
                loop {
                    let curr_size = cmp::min(PAGE_SIZE - offset % PAGE_SIZE, last_len);
                    if curr_size == 0 {
                        break;
                    }
                    let index = offset / PAGE_SIZE;
                    buffer[buffer_off..buffer_off + curr_size].copy_from_slice(
                        inner[index]
                            .0
                            .add(offset % PAGE_SIZE)
                            .slice_with_len(curr_size),
                    );
                    offset += curr_size;
                    last_len -= curr_size;
                    buffer_off += curr_size;
                }
                Ok(read_len)
            }
        }
    }

    fn writeat(&self, mut offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        log::info!("write to ramfs");
        let mut buffer_off = 0;
        let pages = (offset + buffer.len()).div_ceil(PAGE_SIZE);

        let mut inner = self.inner.pages.lock();

        for _ in inner.len()..pages {
            inner.push(frame_alloc().expect("can't alloc frame in ram fs"));
        }

        let mut wsize = buffer.len();
        loop {
            let curr_size = cmp::min(PAGE_SIZE - offset % PAGE_SIZE, wsize);
            if curr_size == 0 {
                break;
            }
            let index = offset / PAGE_SIZE;
            inner[index]
                .0
                .add(offset % PAGE_SIZE)
                .slice_mut_with_len(curr_size)
                .copy_from_slice(&buffer[buffer_off..buffer_off + curr_size]);
            offset += curr_size;
            buffer_off += curr_size;
            wsize -= curr_size;
        }

        let file_size = *self.inner.len.lock();
        if offset > file_size {
            *self.inner.len.lock() = offset;
        }
        Ok(buffer.len())
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        // self.inner.content.lock().drain(size..);
        *self.inner.len.lock() = size;

        log::info!("truncate ramfs:{} insize: {}", size, self.inner.len.lock());

        let mut page_cont = self.inner.pages.lock();
        let pages = page_cont.len();
        // TODO: Check this line.
        let target_pages = size.div_ceil(PAGE_SIZE);

        page_cont.iter().skip(target_pages).for_each(|x| x.clear());

        if size % PAGE_SIZE != 0 {
            let page = size / PAGE_SIZE;
            let offset = size % PAGE_SIZE;
            if let Some(page) = page_cont.get(page) {
                page.0.add(offset).clear_len(PAGE_SIZE - offset);
            }
        }

        for _ in pages..target_pages {
            page_cont.push(frame_alloc().expect("can't alloc frame in ram fs"));
        }

        Ok(())
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        log::debug!("stat ramfs");
        // stat.ino = 1; // TODO: convert path to number(ino)
        if self.inner.name.ends_with(".s") {
            stat.ino = 2; // TODO: convert path to number(ino)
        } else {
            stat.ino = 1; // TODO: convert path to number(ino)
        }
        stat.mode = StatMode::FILE; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        // stat.size = self.inner.content.lock().len() as u64;
        stat.size = *self.inner.len.lock() as u64;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id

        stat.atime = self.inner.times.lock()[1];
        stat.mtime = self.inner.times.lock()[2];
        Ok(())
    }

    fn utimes(&self, times: &mut [TimeSpec]) -> VfsResult<()> {
        if times[0].nsec != UTIME_OMIT {
            self.inner.times.lock()[1] = times[0];
        }
        if times[1].nsec != UTIME_OMIT {
            self.inner.times.lock()[2] = times[1];
        }
        Ok(())
    }
}

impl INodeInterface for RamLink {
    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        // self.link_file.stat(stat)
        stat.ino = self as *const RamLink as u64;
        stat.blksize = 4096;
        stat.blocks = 8;
        stat.size = 3;
        stat.uid = 0;
        stat.gid = 0;
        stat.mode = StatMode::LINK;
        Ok(())
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        self.link_file.readat(offset, buffer)
    }

    fn writeat(&self, offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        self.link_file.writeat(offset, buffer)
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        self.link_file.truncate(size)
    }
}
