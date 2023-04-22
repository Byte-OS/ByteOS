#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

bitflags::bitflags! {
    #[derive(Debug)]
    pub struct OpenFlags: usize {
        // reserve 3 bits for the access mode
        const O_ACCMODE =  0x0007;
        const O_EXEC    =  1;
        const O_RDONLY  =  2;
        const O_RDWR    =  3;
        const O_SEARCH  =  4;
        const O_WRONLY  =  5;

        // these flags get their own bit
        const O_APPEND    = 0x000008;
        const O_CREAT     = 0x40;
        const O_DIRECTORY = 0x0200000;
        const O_EXCL      = 0x000040;
        const O_NOCTTY    = 0x000080;
        const O_NOFOLLOW  = 0x000100;
        const O_TRUNC     = 0x000200;
        const O_NONBLOCK  = 0x000400;
        const O_DSYNC     = 0x000800;
        const O_RSYNC     = 0x001000;
        const O_SYNC      = 0x002000;
        const O_CLOEXEC   = 0x004000;
        const O_PATH      = 0x008000;
        const O_LARGEFILE = 0x010000;
        const O_NOATIME   = 0x020000;
        const O_ASYNC     = 0x040000;
        const O_TMPFILE   = 0x080000;
        const O_DIRECT    = 0x100000;
    }
}

bitflags::bitflags! {
    pub struct MMapFlags: usize {
        const MAP_PRIVATE = 0x1;
        const MAP_SHARED = 0x2;
        const MAP_FIXED = 0x4;
        const MAP_ANONYOMUS = 0x8;
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VfsError {
    NotLinkFile,
    NotDir,
    NotFile,
    NotSupported,
    FileNotFound,
    AlreadyExists,
    InvalidData,
    DirectoryNotEmpty,
    InvalidInput,
    StorageFull,
    UnexpectedEof,
    WriteZero,
    Io,
    Blocking,
    NoMountedPoint,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FileType {
    File,
    Directory,
    Device,
    Socket,
    Link,
}

#[derive(Debug, Copy, Clone)]
pub enum SeekFrom {
    SET(usize),
    CURRENT(isize),
    END(isize),
}

#[derive(Debug, Clone)]
pub struct Metadata<'a> {
    pub filename: &'a str,
    pub inode: usize,
    pub file_type: FileType,
    pub size: usize,
    pub childrens: usize,
}

pub struct DirEntry {
    pub filename: String,
    pub len: usize,
    pub file_type: FileType,
}

#[derive(Clone, Default)]
pub struct MountedInfo {
    pub fs_id: usize,
    pub path: Arc<String>,
}

pub trait FileSystem: Send + Sync {
    fn root_dir(&'static self, mi: MountedInfo) -> Arc<dyn INodeInterface>;
    fn name(&self) -> &str;
    fn flush(&self) -> VfsResult<()> {
        Err(VfsError::FileNotFound)
    }
}

pub type VfsResult<T> = core::result::Result<T, VfsError>;

#[repr(C)]
#[derive(Default)]
pub struct TimeSpec {
    pub sec: usize,  /* 秒 */
    pub nsec: usize, /* 纳秒, 范围在0~999999999 */
}

#[repr(C)]
pub struct Stat {
    pub dev: u64,        // 设备号
    pub ino: u64,        // inode
    pub mode: u32,       // 设备mode
    pub nlink: u32,      // 文件links
    pub uid: u32,        // 文件uid
    pub gid: u32,        // 文件gid
    pub rdev: u64,       // 文件rdev
    pub __pad: u64,      // 保留
    pub size: u64,       // 文件大小
    pub blksize: u32,    // 占用块大小
    pub __pad2: u32,     // 保留
    pub blocks: u64,     // 占用块数量
    pub atime: TimeSpec, // 最后访问时间
    pub mtime: TimeSpec, // 最后修改时间
    pub ctime: TimeSpec, // 最后创建时间
}

#[repr(C)]
pub struct StatFS {
    pub ftype: u64,   // 文件系统的类型
    pub bsize: u64,   // 经优化后的传输块的大小
    pub blocks: u64,  // 文件系统数据块总数
    pub bfree: u64,   // 可用块数
    pub bavail: u64,  // 普通用户能够获得的块数
    pub files: u64,   // 文件结点总数
    pub ffree: u64,   // 可用文件结点数
    pub fsid: u64,    // 文件系统标识
    pub namelen: u64, // 文件名的最大长度
}

#[repr(C)]
pub struct Dirent {
    pub ino: u64,      // 索引结点号
    pub off: i64,      // 到下一个dirent的偏移
    pub reclen: u16,   // 当前dirent的长度
    pub ftype: u8,     // 文件类型
    pub name: [u8; 0], // 文件名
}

pub trait INodeInterface: Send + Sync {
    fn metadata(&self) -> VfsResult<Metadata> {
        Err(VfsError::NotSupported)
    }

    fn read(&self, _buffer: &mut [u8]) -> VfsResult<usize> {
        Err(VfsError::NotFile)
    }

    fn write(&self, _buffer: &[u8]) -> VfsResult<usize> {
        Err(VfsError::NotFile)
    }

    fn seek(&self, _seek: SeekFrom) -> VfsResult<usize> {
        Err(VfsError::NotFile)
    }

    fn mkdir(&self, _name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        Err(VfsError::NotDir)
    }

    fn rmdir(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotDir)
    }

    fn remove(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotDir)
    }

    fn touch(&self, _name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        Err(VfsError::NotDir)
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Err(VfsError::NotDir)
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        Err(VfsError::NotDir)
    }

    fn open(&self, _name: &str, _flags: OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        Err(VfsError::NotDir)
    }

    fn ioctl(&self, _command: usize, _arg: usize) -> VfsResult<usize> {
        Err(VfsError::NotSupported)
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn fcntl(&self, _cmd: usize, _arg: usize) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn flush(&self) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn resolve_link(&self) -> VfsResult<String> {
        Err(VfsError::NotSupported)
    }

    fn link(&self, _name: &str, _src: Arc<dyn INodeInterface>) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn unlink(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn mmap(&self, _offset: usize, _size: usize, _flags: MMapFlags) -> VfsResult<usize> {
        Err(VfsError::NotSupported)
    }

    fn path(&self) -> VfsResult<String> {
        Err(VfsError::NotSupported)
    }

    fn stat(&self, _stat: &mut Stat) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn mount(&self, _path: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn umount(&self) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn statfs(&self, _statfs: &mut StatFS) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn getdents(&self, _buffer: &mut [u8]) -> VfsResult<usize> {
        Err(VfsError::NotSupported)
    }
}
