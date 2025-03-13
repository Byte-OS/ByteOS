use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use devices::get_blk_device;

use sync::Mutex;
use vfscore::{
    DirEntry, FileSystem, FileType, INodeInterface, Metadata, OpenFlags, StatFS, StatMode,
    TimeSpec, VfsResult,
};

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
        // log::info!("write_offset: {:x?} buf_len{:x?}", offset, buf.len());
        let device = get_blk_device(self.device_id).unwrap();

        let start_block_id = offset / 512;
        let mut offset_in_block = offset % 512;

        // assert_eq!(offset_in_block, 0);

        let bytes_to_write = buf.len();
        let mut total_bytes_written = 0;

        for i in 0..((bytes_to_write + 511) / 512) {
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
    fn root_dir(&'static self) -> Arc<dyn INodeInterface> {
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
        let ext4 = Ext4::open(disk);

        let root = Arc::new(Ext4FileWrapper::load_root(ext4.clone()));
        Arc::new(Self {
            inner: ext4,
            root,
            file_type: FileType::Directory,
        })
    }
}

pub struct Ext4FileWrapper {
    inner: Mutex<Ext4File>,
    ext4: Arc<Ext4>,
    file_type: FileType,
    file_name: String,
}

impl Ext4FileWrapper {
    fn load_root(ext4: Arc<Ext4>) -> Self {
        let mut ext4_file = Ext4File::new();
        let _ = ext4.ext4_open(&mut ext4_file, "/", "r", false);

        Self {
            inner: Mutex::new(ext4_file),
            ext4,
            file_type: FileType::Directory,
            file_name: "/".to_string(),
        }
    }
}

impl INodeInterface for Ext4FileWrapper {
    fn open(&self, path: &str, flags: vfscore::OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        let mut ext4_file = Ext4File::new();

        let mut create = false;

        if flags.contains(OpenFlags::O_CREAT) {
            create = true;
        };
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

        let r = self.ext4.ext4_open(&mut ext4_file, path, "r+", create);

        if let Err(e) = r {
            match e.error() {
                Errnum::ENOENT => Err(vfscore::VfsError::FileNotFound),
                Errnum::EALLOCFIAL => Err(vfscore::VfsError::UnexpectedEof),
                Errnum::ELINKFIAL => Err(vfscore::VfsError::UnexpectedEof),

                _ => Err(vfscore::VfsError::UnexpectedEof),
            }
        } else {
            Ok(Arc::new(Ext4FileWrapper {
                inner: Mutex::new(ext4_file),
                ext4: self.ext4.clone(),
                file_type: self.file_type,
                file_name: String::from(path),
            }))
        }
    }

    fn mkdir(&self, path: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        let _ = self.ext4.ext4_dir_mk(path);
        let mut ext4_file = Ext4File::new();

        let _ = self.ext4.ext4_open(&mut ext4_file, path, "w", false);

        Ok(Arc::new(Ext4FileWrapper {
            inner: Mutex::new(ext4_file),
            ext4: self.ext4.clone(),
            file_type: FileType::Directory,
            file_name: String::from(path),
        }))
    }

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        if self.file_name == "/" {
            return Ok(Metadata {
                filename: "/",
                // root
                inode: 2 as usize,
                // dir
                file_type: FileType::Directory,
                size: 0,
                childrens: 0,
            });
        }

        let mut ext4_file = Ext4File::new();
        let _ = self
            .ext4
            .ext4_open(&mut ext4_file, &self.file_name, "r+", false);

        Ok(Metadata {
            filename: &self.file_name,
            inode: ext4_file.inode as usize,
            file_type: self.file_type,
            size: ext4_file.fsize as _,
            childrens: 0,
        })
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        let mut ext4_file = self.inner.lock();

        ext4_file.fpos = offset;

        let read_len = buffer.len();
        let mut read_cnt = 0;

        let r = self
            .ext4
            .ext4_file_read(&mut ext4_file, buffer, read_len, &mut read_cnt);

        if let Err(e) = r {
            match e.error() {
                Errnum::EINVAL => Err(vfscore::VfsError::InvalidInput),
                _ => Err(vfscore::VfsError::UnexpectedEof),
            }
        } else {
            Ok(ext4_file.fpos - offset)
        }
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

    fn touch(&self, path: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        let mut ext4_file = Ext4File::new();
        let _ = self.ext4.ext4_open(&mut ext4_file, path, "w+", true);
        Ok(Arc::new(Ext4FileWrapper {
            inner: Mutex::new(ext4_file),
            ext4: self.ext4.clone(),
            file_type: FileType::File,
            file_name: String::from(path),
        }))
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        let ext4file = self.inner.lock();
        let mut inode_num = ext4file.inode;
        if inode_num == 0 && self.file_name == "/" {
            inode_num = 2;
        }
        let v: Vec<Ext4DirEntry> = self.ext4.read_dir_entry(inode_num as _);

        let mut entries = Vec::new();

        for i in v.iter() {
            let file_type =
                map_ext4_type(unsafe { DirEntryType::from_bits(i.inner.inode_type).unwrap() });

            let entry = DirEntry {
                filename: i.get_name(),
                len: i.entry_len as usize,
                file_type,
            };

            entries.push(entry);
        }
        Ok(entries)
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        todo!("ext4 loopup")
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Ok(())
    }

    fn resolve_link(&self) -> VfsResult<alloc::string::String> {
        Err(vfscore::VfsError::NotSupported)
    }

    fn link(&self, _name: &str, _src: Arc<dyn INodeInterface>) -> VfsResult<()> {
        Err(vfscore::VfsError::NotSupported)
    }

    fn sym_link(&self, _name: &str, _src: &str) -> VfsResult<()> {
        Err(vfscore::VfsError::NotSupported)
    }

    fn unlink(&self, _name: &str) -> VfsResult<()> {
        Ok(())
    }

    fn stat(&self, stat: &mut vfscore::Stat) -> VfsResult<()> {
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = match self.file_type {
            FileType::File => StatMode::FILE,
            FileType::Directory => StatMode::DIR,
            FileType::Device => StatMode::BLOCK,
            FileType::Socket => StatMode::SOCKET,
            FileType::Link => StatMode::LINK,
        };
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = self.inner.lock().fsize as _;
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

pub fn map_ext4_type(value: DirEntryType) -> FileType {
    match value {
        DirEntryType::EXT4_DE_REG_FILE => FileType::File,
        DirEntryType::EXT4_DE_DIR => FileType::Directory,
        DirEntryType::EXT4_DE_CHRDEV => FileType::Device,
        DirEntryType::EXT4_DE_BLKDEV => FileType::Device,
        DirEntryType::EXT4_DE_SOCK => FileType::Socket,
        DirEntryType::EXT4_DE_SYMLINK => FileType::Link,
        _ => FileType::File,
    }
}
