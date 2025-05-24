#![no_std]
extern crate alloc;

extern crate bitflags;

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use downcast_rs::{impl_downcast, DowncastSync};
use libc_types::{
    poll::PollEvent,
    types::{Stat, StatFS, StatMode, TimeSpec},
};
use syscalls::Errno;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FileType {
    File,
    Directory,
    Device,
    Socket,
    Link,
}

impl From<StatMode> for FileType {
    fn from(value: StatMode) -> Self {
        match value.intersection(StatMode::TYPE_MASK) {
            StatMode::SOCKET => FileType::Socket,
            StatMode::LINK => FileType::Link,
            StatMode::FILE => FileType::File,
            StatMode::BLOCK => FileType::Device,
            StatMode::DIR => FileType::Directory,
            StatMode::CHAR => FileType::Device,
            StatMode::FIFO => FileType::Device,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SeekFrom {
    SET(usize),
    CURRENT(isize),
    END(isize),
}

pub struct DirEntry {
    pub filename: String,
    pub len: usize,
    pub file_type: FileType,
}

pub trait FileSystem: Send + Sync {
    fn root_dir(&self) -> Arc<dyn INodeInterface>;
    fn name(&self) -> &str;
    fn flush(&self) -> VfsResult<()> {
        Ok(())
    }
}

pub type VfsResult<T> = core::result::Result<T, Errno>;

pub trait BlockDevice: Send + Sync {
    fn read_block(&self, block: usize, buffer: &mut [u8]) -> VfsResult<usize>;
    fn write_block(&self, block: usize, buffer: &[u8]) -> VfsResult<usize>;
    fn capacity(&self) -> VfsResult<usize>;
}

pub trait INodeInterface: DowncastSync + Send + Sync {
    fn readat(&self, _offset: usize, _buffer: &mut [u8]) -> VfsResult<usize> {
        Err(Errno::ENOENT)
    }

    fn writeat(&self, _offset: usize, _buffer: &[u8]) -> VfsResult<usize> {
        Err(Errno::ENOENT)
    }

    fn create(&self, _name: &str, _ty: FileType) -> VfsResult<()> {
        Err(Errno::ENOTDIR)
    }

    fn mkdir(&self, _name: &str) -> VfsResult<()> {
        Err(Errno::EEXIST)
    }

    fn rmdir(&self, _name: &str) -> VfsResult<()> {
        Err(Errno::ENOENT)
    }

    fn remove(&self, _name: &str) -> VfsResult<()> {
        Err(Errno::ENOENT)
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Err(Errno::EPERM)
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        Err(Errno::ENOENT)
    }

    fn ioctl(&self, _command: usize, _arg: usize) -> VfsResult<usize> {
        Err(Errno::EPERM)
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn flush(&self) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn resolve_link(&self) -> VfsResult<String> {
        Err(Errno::EPERM)
    }

    fn link(&self, _name: &str, _src: Arc<dyn INodeInterface>) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn symlink(&self, _name: &str, _src: &str) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn unlink(&self, _name: &str) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn stat(&self, _stat: &mut Stat) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn mount(&self, _path: &str) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn umount(&self) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn statfs(&self, _statfs: &mut StatFS) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn utimes(&self, _times: &mut [TimeSpec]) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn poll(&self, _events: PollEvent) -> VfsResult<PollEvent> {
        Err(Errno::EPERM)
    }
}

impl_downcast!(sync INodeInterface);
