#![no_std]

#[macro_use]
extern crate alloc;

use alloc::{sync::Arc, vec::Vec};
use devices::get_blk_device;
use libc_types::types::{Stat, StatFS, StatMode, TimeSpec};
use syscalls::Errno;
use vfscore::{DirEntry, FileSystem, FileType, INodeInterface, VfsResult};

use ext4_rs::*;

const BLOCK_SIZE: usize = 4096;

#[derive(Debug)]
pub struct Ext4Disk {
    device_id: usize,
}

impl Ext4Disk {
    /// Create a new disk.
    pub fn new(device_id: usize) -> Self {
        Self { device_id }
    }
}

impl BlockDevice for Ext4Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        let mut buf = vec![0; BLOCK_SIZE];
        let device = get_blk_device(self.device_id).unwrap();

        let start_block_id = offset / 512;
        let mut offset_in_block = offset % 512;

        let mut total_bytes_read = 0;
        for i in 0..(BLOCK_SIZE / 512) {
            let mut data = vec![0u8; 512];
            let current_block_id = start_block_id + i;

            device.read_blocks(current_block_id, &mut data);
            let bytes_to_copy = if total_bytes_read == 0 {
                512 - offset_in_block
            } else {
                512
            };

            let buf_start = total_bytes_read;
            let buf_end = buf_start + bytes_to_copy;
            buf[buf_start..buf_end]
                .copy_from_slice(&data[offset_in_block..(offset_in_block + bytes_to_copy)]);

            total_bytes_read += bytes_to_copy;
            offset_in_block = 0; // only the first block has an offset within the block
        }

        buf
    }

    fn write_offset(&self, offset: usize, buf: &[u8]) {
        let device = get_blk_device(self.device_id).unwrap();

        let start_block_id = offset / 512;
        let mut offset_in_block = offset % 512;

        // assert_eq!(offset_in_block, 0);

        let bytes_to_write = buf.len();
        let mut total_bytes_written = 0;

        for i in 0..bytes_to_write.div_ceil(512) {
            // round up to cover partial blocks
            let current_block_id = start_block_id + i;
            let mut data = [0u8; 512];

            if bytes_to_write < 512 {
                // Read the current block data first if we're writing less than a full block
                device.read_blocks(current_block_id, &mut data);
            }

            let buf_start = total_bytes_written;
            let buf_end = if buf_start + 512 > bytes_to_write {
                bytes_to_write
            } else {
                buf_start + 512
            };
            let bytes_to_copy = buf_end - buf_start;

            data[offset_in_block..offset_in_block + bytes_to_copy]
                .copy_from_slice(&buf[buf_start..buf_end]);
            device.write_blocks(current_block_id as usize, &data);

            total_bytes_written += bytes_to_copy;
            offset_in_block = 0; // only the first block has an offset within the block
        }
    }
}

/// TODO: use inner fields AND Fix some warnings.
#[allow(dead_code)]
pub struct Ext4FileSystem {
    inner: Arc<Ext4>,
    root: Arc<dyn INodeInterface>,
    file_type: FileType,
    // file_name: String,
}

impl FileSystem for Ext4FileSystem {
    fn root_dir(&self) -> Arc<dyn INodeInterface> {
        self.root.clone()
    }

    fn name(&self) -> &str {
        "ext4"
    }
}

unsafe impl Sync for Ext4FileSystem {}
unsafe impl Send for Ext4FileSystem {}

impl Ext4FileSystem {
    pub fn new(device_id: usize) -> Arc<Self> {
        let disk = Arc::new(Ext4Disk::new(device_id));
        let ext4 = Arc::new(Ext4::open(disk));

        let root = Arc::new(Ext4FileWrapper::load_root(ext4.clone()));
        Arc::new(Self {
            inner: ext4,
            root,
            file_type: FileType::Directory,
        })
    }
}

pub struct Ext4FileWrapper {
    inode: u32,
    ext4: Arc<Ext4>,
}

impl Ext4FileWrapper {
    fn load_root(ext4: Arc<Ext4>) -> Self {
        // let inode = ext4.ext4_dir_open("/").unwrap();
        let inode = 2;
        log::debug!("root inode: {}", inode);
        Self { inode, ext4 }
    }
}

impl INodeInterface for Ext4FileWrapper {
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        log::debug!("lookup file: {}", name);
        self.ext4
            .dir_get_entries(self.inode)
            .iter()
            .find(|x| &x.name[..x.name_len as usize] == name.as_bytes());
        let mut inode = self.inode;
        let inode = self
            .ext4
            .generic_open(name, &mut inode, false, 0, &mut 0)
            .map_err(|x| Errno::new(x.error() as _))?;
        // let mut parse_flags: &str;
        // match flags {
        //     OpenFlags::O_RDONLY => parse_flags = "r",
        //     OpenFlags::O_WRONLY | OpenFlags::O_CREAT | OpenFlags::O_TRUNC => parse_flags = "w",
        //     OpenFlags::O_WRONLY | OpenFlags::O_CREAT | OpenFlags::O_APPEND => parse_flags = "a",
        //     OpenFlags::O_RDWR => parse_flags = "r+",
        //     OpenFlags::O_RDWR | OpenFlags::O_CREAT | OpenFlags::O_TRUNC => parse_flags = "w+",
        //     OpenFlags::O_RDWR | OpenFlags::O_CREAT | OpenFlags::O_APPEND => parse_flags = "a+",
        //     _ => parse_flags = "r+",
        // };

        Ok(Arc::new(Ext4FileWrapper {
            ext4: self.ext4.clone(),
            inode,
        }))
    }

    fn mkdir(&self, path: &str) -> VfsResult<()> {
        // todo!("mkdir ");
        log::debug!("self inode: {}  create {}", self.inode, path);
        let mut inode = self.inode;
        let mut name_off = 0;
        // self.ext4
        //     .generic_open(
        //         path,
        //         &mut inode,
        //         true,
        //         InodeFileType::S_IFDIR.bits(),
        //         &mut name_off,
        //     )
        //     .map_err(map_ext4_err)?;
        // self.ext4.dir_mk(path).map_err(map_ext4_err)?;
        self.ext4
            .create(self.inode, path, InodeFileType::S_IFDIR.bits())
            .map_err(map_ext4_err)?;
        log::debug!("mkdir done");
        Ok(())
    }

    fn create(&self, _name: &str, _ty: FileType) -> VfsResult<()> {
        panic!("create")
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        self.ext4
            .read_at(self.inode, offset, buffer)
            .map_err(map_ext4_err)
    }

    fn writeat(&self, _offset: usize, _buffer: &[u8]) -> VfsResult<usize> {
        todo!("ext4 loopup")
    }

    fn rmdir(&self, _name: &str) -> VfsResult<()> {
        todo!("ext4 loopup")
    }

    fn remove(&self, _name: &str) -> VfsResult<()> {
        todo!("ext4 loopup")
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        todo!("readdir")
        // let ext4file = self.inner.lock();
        // let mut inode_num = ext4file.inode;
        // if inode_num == 0 && self.file_name == "/" {
        //     inode_num = 2;
        // }
        // let v: Vec<Ext4DirEntry> = self.ext4.read_dir_entry(inode_num as _);

        // let mut entries = Vec::new();

        // for i in v.iter() {
        //     let file_type =
        //         map_ext4_type(unsafe { DirEntryType::from_bits(i.inner.inode_type).unwrap() });

        //     let entry = DirEntry {
        //         filename: i.get_name(),
        //         len: i.entry_len as usize,
        //         file_type,
        //     };

        //     entries.push(entry);
        // }
        // Ok(entries)
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Ok(())
    }

    fn resolve_link(&self) -> VfsResult<alloc::string::String> {
        Err(Errno::EPERM)
    }

    fn link(&self, _name: &str, _src: Arc<dyn INodeInterface>) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn symlink(&self, _name: &str, _src: &str) -> VfsResult<()> {
        Err(Errno::EPERM)
    }

    fn unlink(&self, _name: &str) -> VfsResult<()> {
        Ok(())
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.ino = 1; // TODO: convert path to number(ino)
        let inodeif = self.ext4.get_inode_ref(self.inode);
        stat.mode = match inodeif.inode.file_type() {
            InodeFileType::S_IFREG => StatMode::FILE,
            InodeFileType::S_IFDIR => StatMode::DIR,
            InodeFileType::S_IFBLK => StatMode::BLOCK,
            InodeFileType::S_IFSOCK => StatMode::SOCKET,
            InodeFileType::S_IFLNK => StatMode::LINK,
            InodeFileType::S_IFCHR => StatMode::CHAR,
            InodeFileType::S_IFIFO => StatMode::FIFO,
            _ => unreachable!(),
        };
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = inodeif.inode.size();
        stat.blksize = 4096;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        stat.atime.nsec = 0;
        stat.atime.sec = 0;
        stat.ctime.nsec = 0;
        stat.ctime.sec = 0;
        stat.mtime.nsec = 0;
        stat.mtime.sec = 0;
        Ok(())
    }

    fn statfs(&self, statfs: &mut StatFS) -> VfsResult<()> {
        statfs.ftype = 32;
        statfs.bsize = 4096;
        statfs.blocks = 80;
        statfs.bfree = 40;
        statfs.bavail = 0;
        statfs.files = 32;
        statfs.ffree = 0;
        statfs.fsid = 32;
        statfs.namelen = 20;
        Ok(())
    }

    fn utimes(&self, _times: &mut [TimeSpec]) -> VfsResult<()> {
        Ok(())
    }
}

#[inline(always)]
pub fn map_ext4_err(err: Ext4Error) -> Errno {
    Errno::new(err.error() as _)
}
