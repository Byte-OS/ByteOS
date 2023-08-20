#![no_std]
#![feature(drain_filter)]

#[macro_use]
extern crate alloc;

use core::{
    cmp::{self, min},
    mem::size_of,
};

use alloc::{string::String, sync::Arc, vec::Vec};
use arch::PAGE_SIZE;
use frame_allocator::{ceil_div, frame_alloc, FrameTracker};
use sync::Mutex;
use vfscore::{
    DirEntry, Dirent64, FileSystem, FileType, INodeInterface, Metadata, Stat, StatMode, TimeSpec,
    VfsError, VfsResult, UTIME_OMIT,
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
    fn root_dir(&'static self) -> Arc<dyn INodeInterface> {
        Arc::new(RamDir {
            inner: self.root.clone(),
            dents_off: Mutex::new(0),
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
            FileContainer::Dir(dir) => Ok(Arc::new(RamDir {
                inner: dir.clone(),
                dents_off: Mutex::new(0),
            })),
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
    dents_off: Mutex<usize>,
}

impl INodeInterface for RamDir {
    fn open(&self, name: &str, _flags: vfscore::OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map(|x| x.to_inode())
            .ok_or(VfsError::FileNotFound)?
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
            // content: Mutex::new(Vec::new()),
            times: Mutex::new([Default::default(); 3]),
            len: Mutex::new(0),
            pages: Mutex::new(vec![]),
        });

        let new_file = Arc::new(RamFile {
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
            dents_off: Mutex::new(0),
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
                FileContainer::Dir(x) => x.name == name,
                _ => false,
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
            .drain_filter(|x| match x {
                FileContainer::File(x) => x.name == name,
                FileContainer::Dir(_) => false,
                FileContainer::Link(x) => x.name == name,
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

    fn getdents(&self, buffer: &mut [u8]) -> VfsResult<usize> {
        let buf_ptr = buffer.as_mut_ptr() as usize;
        let len = buffer.len();
        let mut ptr: usize = buf_ptr;
        let mut finished = 0;
        for (i, x) in self
            .inner
            .children
            .lock()
            .iter()
            .enumerate()
            .skip(*self.dents_off.lock())
        {
            let filename = x.filename();
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
        *self.dents_off.lock() = finished;
        Ok(ptr - buf_ptr)
    }

    fn link(&self, name: &str, src: Arc<dyn INodeInterface>) -> VfsResult<()> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(VfsError::AlreadyExists))?;

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
                    let page_data = inner[index].0.get_buffer();
                    buffer[buffer_off..buffer_off + curr_size].copy_from_slice(
                        &page_data[offset % PAGE_SIZE..offset % PAGE_SIZE + curr_size],
                    );
                    offset += curr_size;
                    last_len -= curr_size;
                    buffer_off += curr_size;
                }
                // Ok(origin_read_len)
                Ok(read_len)
            }
        }
    }

    fn writeat(&self, mut offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        let mut buffer_off = 0;
        let pages = ceil_div(offset + buffer.len(), PAGE_SIZE);

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
            let page_data = inner[index].0.get_buffer();
            page_data[offset % PAGE_SIZE..offset % PAGE_SIZE + curr_size]
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

        log::debug!("truncate ramfs:{} insize: {}", size, self.inner.len.lock());

        let mut page_cont = self.inner.pages.lock();
        let pages = page_cont.len();
        let target_pages = ceil_div(size, PAGE_SIZE);

        let curr_page = ceil_div(size, PAGE_SIZE);
        page_cont
            .iter()
            .skip(curr_page)
            .for_each(|x| x.0.drop_clear());

        if size % PAGE_SIZE != 0 {
            let page = size / PAGE_SIZE;
            let offset = size % PAGE_SIZE;
            if let Some(page) = page_cont.get(page) {
                page.0.get_buffer()[offset..].fill(0);
            }
        }

        for _ in pages..target_pages {
            page_cont.push(frame_alloc().expect("can't alloc frame in ram fs"));
        }

        Ok(())
    }

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        Ok(Metadata {
            filename: &self.inner.name,
            inode: 0,
            file_type: FileType::File,
            // size: self.inner.content.lock().len(),
            size: *self.inner.len.lock(),
            childrens: 0,
        })
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

impl INodeInterface for RamLink {
    fn metadata(&self) -> VfsResult<Metadata> {
        self.link_file.metadata()
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        self.link_file.stat(stat)
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        self.link_file.readat(offset, buffer)
    }
}
