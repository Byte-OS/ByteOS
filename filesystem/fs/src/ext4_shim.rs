use core::iter::zip;

use alloc::{
    ffi::CString,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use devices::get_blk_device;
use lwext4_rust::{
    bindings::{O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY},
    Ext4BlockWrapper, Ext4File, InodeTypes, KernelDevOp,
};
use sync::Mutex;
use vfscore::{
    DirEntry, FileSystem, FileType, INodeInterface, Metadata, OpenFlags, StatFS, StatMode,
    TimeSpec, VfsError, VfsResult,
};

const BLOCK_SIZE: usize = 0x200;

pub struct Ext4DiskWrapper {
    block_id: usize,
    offset: usize,
    blk_id: usize,
}

impl Ext4DiskWrapper {
    /// Create a new disk.
    pub fn new(blk_id: usize) -> Self {
        Self {
            block_id: 0,
            offset: 0,
            blk_id,
        }
    }

    /// Get the position of the cursor.
    pub fn position(&self) -> u64 {
        (self.block_id * BLOCK_SIZE + self.offset) as u64
    }

    /// Set the position of the cursor.
    pub fn set_position(&mut self, pos: u64) {
        self.block_id = pos as usize / BLOCK_SIZE;
        self.offset = pos as usize % BLOCK_SIZE;
    }

    /// Read within one block, returns the number of bytes read.
    pub fn read_one(&mut self, buf: &mut [u8]) -> Result<usize, i32> {
        // info!("block id: {}", self.block_id);
        let read_size = if self.offset == 0 && buf.len() >= BLOCK_SIZE {
            // whole block
            get_blk_device(self.blk_id)
                .expect("can't find block device")
                .read_blocks(self.block_id, &mut buf[0..BLOCK_SIZE]);
            self.block_id += 1;
            BLOCK_SIZE
        } else {
            // partial block
            let mut data = [0u8; BLOCK_SIZE];
            let start = self.offset;
            let count = buf.len().min(BLOCK_SIZE - self.offset);
            if start > BLOCK_SIZE {
                info!("block size: {} start {}", BLOCK_SIZE, start);
            }

            get_blk_device(self.blk_id)
                .expect("can't find block device")
                .read_blocks(self.block_id, &mut data);
            buf[..count].copy_from_slice(&data[start..start + count]);

            self.offset += count;
            if self.offset >= BLOCK_SIZE {
                self.block_id += 1;
                self.offset -= BLOCK_SIZE;
            }
            count
        };
        Ok(read_size)
    }

    /// Write within one block, returns the number of bytes written.
    pub fn write_one(&mut self, buf: &[u8]) -> Result<usize, i32> {
        let write_size = if self.offset == 0 && buf.len() >= BLOCK_SIZE {
            // whole block
            get_blk_device(self.blk_id)
                .expect("can't find block device")
                .write_blocks(self.block_id, &buf[0..BLOCK_SIZE]);
            self.block_id += 1;
            BLOCK_SIZE
        } else {
            // partial block
            let mut data = [0u8; BLOCK_SIZE];
            let start = self.offset;
            let count = buf.len().min(BLOCK_SIZE - self.offset);

            get_blk_device(self.blk_id)
                .expect("can't find block device")
                .read_blocks(self.block_id, &mut data);
            data[start..start + count].copy_from_slice(&buf[..count]);
            get_blk_device(self.blk_id)
                .expect("can't find block device")
                .write_blocks(self.block_id, &data);

            self.offset += count;
            if self.offset >= BLOCK_SIZE {
                self.block_id += 1;
                self.offset -= BLOCK_SIZE;
            }
            count
        };
        Ok(write_size)
    }
}

impl KernelDevOp for Ext4DiskWrapper {
    type DevType = Self;

    fn write(dev: &mut Self::DevType, mut buf: &[u8]) -> Result<usize, i32> {
        let mut write_len = 0;
        while !buf.is_empty() {
            match dev.write_one(buf) {
                Ok(0) => break,
                Ok(n) => {
                    buf = &buf[n..];
                    write_len += n;
                }
                Err(_e) => return Err(-1),
            }
        }
        Ok(write_len)
    }

    fn read(dev: &mut Self::DevType, mut buf: &mut [u8]) -> Result<usize, i32> {
        let mut read_len = 0;
        while !buf.is_empty() {
            match dev.read_one(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                    read_len += n;
                }
                Err(_e) => return Err(-1),
            }
        }
        Ok(read_len)
    }

    fn seek(dev: &mut Self::DevType, off: i64, whence: i32) -> Result<i64, i32> {
        // let size = dev.size();
        let size = get_blk_device(dev.blk_id)
            .expect("can't seek to device")
            .capacity();
        let new_pos = match whence as u32 {
            lwext4_rust::bindings::SEEK_SET => Some(off),
            lwext4_rust::bindings::SEEK_CUR => {
                dev.position().checked_add_signed(off).map(|v| v as i64)
            }
            lwext4_rust::bindings::SEEK_END => size.checked_add_signed(off as _).map(|v| v as i64),
            _ => Some(off),
        }
        .ok_or(-1)?;

        if new_pos as u64 > (size as _) {
            log::warn!("Seek beyond the end of the block device");
        }
        dev.set_position(new_pos as u64);
        Ok(new_pos)
    }

    fn flush(_dev: &mut Self::DevType) -> Result<usize, i32>
    where
        Self: Sized,
    {
        todo!()
    }
}

pub struct Ext4FileSystem {
    _inner: Ext4BlockWrapper<Ext4DiskWrapper>,
    root: Arc<dyn INodeInterface>,
}

unsafe impl Sync for Ext4FileSystem {}
unsafe impl Send for Ext4FileSystem {}

impl Ext4FileSystem {
    pub fn new(blk_id: usize) -> Arc<Self> {
        let disk = Ext4DiskWrapper::new(blk_id);
        info!("Got position:{}", disk.position());
        let inner = Ext4BlockWrapper::<Ext4DiskWrapper>::new(disk)
            .expect("failed to initialize EXT4 filesystem");
        let root = Arc::new(Ext4FileWrapper::new("/", InodeTypes::EXT4_DE_DIR));
        Arc::new(Self {
            _inner: inner,
            root,
        })
    }
}

fn map_ext4_err(err: i32) -> VfsError {
    match err {
        2 => VfsError::FileNotFound,
        _ => VfsError::NotSupported,
    }
}

impl FileSystem for Ext4FileSystem {
    fn root_dir(&'static self) -> Arc<dyn INodeInterface> {
        self.root.clone()
    }

    fn name(&self) -> &str {
        "ext4"
    }
}

pub struct Ext4FileWrapper {
    filename: String,
    file_type: FileType,
    inner: Mutex<Ext4File>,
}

impl Ext4FileWrapper {
    fn new(path: &str, types: InodeTypes) -> Self {
        //file.file_read_test("/test/test.txt", &mut buf);
        let file = Ext4File::new(path, types);
        let file_type = map_ext4_type(file.get_type());
        Self {
            filename: file
                .get_path()
                .to_str()
                .expect("can't convert file")
                .to_string(),
            file_type,
            inner: Mutex::new(file),
        }
    }
}

unsafe impl Send for Ext4FileWrapper {}
unsafe impl Sync for Ext4FileWrapper {}

pub fn map_ext4_type(value: InodeTypes) -> FileType {
    match value {
        InodeTypes::EXT4_DE_UNKNOWN => FileType::File,
        InodeTypes::EXT4_DE_REG_FILE => FileType::File,
        InodeTypes::EXT4_DE_DIR => FileType::Directory,
        InodeTypes::EXT4_DE_CHRDEV => FileType::Device,
        InodeTypes::EXT4_DE_BLKDEV => FileType::Device,
        InodeTypes::EXT4_DE_FIFO => FileType::Device,
        InodeTypes::EXT4_DE_SOCK => FileType::Socket,
        InodeTypes::EXT4_DE_SYMLINK => FileType::Link,
        InodeTypes::EXT4_INODE_MODE_FIFO => todo!(),
        InodeTypes::EXT4_INODE_MODE_CHARDEV => todo!(),
        InodeTypes::EXT4_INODE_MODE_DIRECTORY => todo!(),
        InodeTypes::EXT4_INODE_MODE_BLOCKDEV => todo!(),
        InodeTypes::EXT4_INODE_MODE_FILE => todo!(),
        InodeTypes::EXT4_INODE_MODE_SOFTLINK => todo!(),
        InodeTypes::EXT4_INODE_MODE_SOCKET => todo!(),
        InodeTypes::EXT4_INODE_MODE_TYPE_MASK => todo!(),
    }
}

impl Ext4FileWrapper {
    fn path_deal_with(&self, path: &str) -> String {
        if path.starts_with('/') {
            log::warn!("path_deal_with: {}", path);
        }
        let p = path.trim_matches('/'); // 首尾去除
        if p.is_empty() || p == "." {
            return String::new();
        }

        if let Some(rest) = p.strip_prefix("./") {
            //if starts with "./"
            return self.path_deal_with(rest);
        }
        let rest_p = p.replace("//", "/");
        if p != rest_p {
            return self.path_deal_with(&rest_p);
        }

        //Todo ? ../
        //注：lwext4创建文件必须提供文件path的绝对路径
        let file = self.inner.lock();
        let path = file.get_path();
        let fpath = String::from(path.to_str().unwrap().trim_end_matches('/')) + "/" + p;
        fpath
    }

    #[inline]
    fn file_open(&self, flags: u32) -> VfsResult<()> {
        let mut file = self.inner.lock();
        let path = file.get_path();
        let path = path.to_str().unwrap();
        file.file_open(path, flags).map_err(map_ext4_err)?;
        Ok(())
    }
}

// TIPS: Write a macro or a function to ensure that file type.
// Such as read()
// fn read(...) -> VfsResult<usize> {
//     check_file_type!(FileType::File | FileType::Device, VfsError::InvalidFile)?;
//     // or use regular function
//     check_file_type(FileType::File);
// }
impl INodeInterface for Ext4FileWrapper {
    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        if self.filename == "/" {
            return Ok(Metadata {
                filename: &self.filename,
                inode: self.inner.lock().get_path().into_raw() as usize,
                file_type: self.file_type,
                size: 0,
                childrens: 0,
            });
        }
        self.file_open(O_RDWR)?;
        let mut file = self.inner.lock();
        Ok(Metadata {
            filename: &self.filename,
            inode: file.get_path().into_raw() as usize,
            file_type: self.file_type,
            size: file.file_size() as _,
            childrens: 0,
        })
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        let mut file = self.inner.lock();
        let path = file.get_path();
        let path = path.to_str().unwrap();
        file.file_open(path, O_RDONLY).map_err(map_ext4_err)?;
        file.file_seek(offset as _, 0).map_err(map_ext4_err)?;
        let rsize = file.file_read(buffer).map_err(map_ext4_err)?;
        let _ = file.file_close();
        Ok(rsize)
    }

    fn writeat(&self, offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        let mut file = self.inner.lock();
        let path = file.get_path();
        let path = path.to_str().unwrap();
        file.file_open(path, O_RDONLY).map_err(map_ext4_err)?;
        file.file_seek(offset as _, 0).map_err(map_ext4_err)?;
        let wsize = file.file_write(buffer).map_err(map_ext4_err)?;
        let _ = file.file_close();
        Ok(wsize)
    }

    fn mkdir(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        // let path = format!("{}/{}", self.inner.lock().get_path().to_str().unwrap(), name);
        let fpath = self.path_deal_with(&name);
        self.inner.lock().dir_mk(&fpath).map_err(map_ext4_err)?;
        Ok(Arc::new(Ext4FileWrapper {
            filename: name.to_string(),
            file_type: FileType::Directory,
            inner: Mutex::new(Ext4File::new(&fpath, InodeTypes::EXT4_DE_DIR)),
        }))
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        self.unlink(name)
    }

    fn remove(&self, name: &str) -> VfsResult<()> {
        self.unlink(name)
    }

    fn touch(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        let fpath = self.path_deal_with(name);
        let mut file = self.inner.lock();
        file.file_open(&fpath, O_WRONLY | O_CREAT | O_TRUNC)
            .map_err(map_ext4_err)?;
        file.file_close().map_err(map_ext4_err)?;
        Ok(Arc::new(Ext4FileWrapper {
            filename: name.to_string(),
            file_type: FileType::File,
            inner: Mutex::new(Ext4File::new(&fpath, InodeTypes::EXT4_DE_REG_FILE)),
        }))
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        let iters = self
            .inner
            .lock()
            .lwext4_dir_entries()
            .map_err(map_ext4_err)?;
        let mut ans = Vec::new();
        for (name, file_type) in zip(iters.0, iters.1).skip(3) {
            ans.push(DirEntry {
                filename: CString::from_vec_with_nul(name)
                    .map_err(|_| VfsError::InvalidData)?
                    .to_str()
                    .map_err(|_| VfsError::InvalidData)?
                    .to_string(),
                len: 0,
                file_type: map_ext4_type(file_type),
            })
        }
        Ok(ans)
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        todo!("ext4 loopup")
    }

    fn open(&self, name: &str, flags: vfscore::OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        let fpath = self.path_deal_with(name);
        let mut file = self.inner.lock();
        if file.check_inode_exist(&fpath, InodeTypes::EXT4_DE_DIR) {
            Ok(Arc::new(Ext4FileWrapper {
                filename: name.to_string(),
                file_type: FileType::Directory,
                inner: Mutex::new(Ext4File::new(&fpath, InodeTypes::EXT4_DE_DIR)),
            }))
        } else if file.check_inode_exist(&fpath, InodeTypes::EXT4_DE_REG_FILE) {
            Ok(Arc::new(Ext4FileWrapper {
                filename: name.to_string(),
                file_type: FileType::File,
                inner: Mutex::new(Ext4File::new(&fpath, InodeTypes::EXT4_DE_REG_FILE)),
            }))
        } else {
            if flags.contains(OpenFlags::O_CREAT) {
                if flags.contains(OpenFlags::O_DIRECTORY) {
                    drop(file);
                    self.mkdir(name)
                } else {
                    file.file_open(&fpath, O_WRONLY | O_CREAT | O_TRUNC)
                        .map_err(map_ext4_err)?;
                    file.file_close().map_err(map_ext4_err)?;
                    Ok(Arc::new(Ext4FileWrapper {
                        filename: name.to_string(),
                        file_type: FileType::File,
                        inner: Mutex::new(Ext4File::new(&fpath, InodeTypes::EXT4_DE_REG_FILE)),
                    }))
                }
            } else {
                Err(vfscore::VfsError::FileNotFound)
            }
        }
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        self.file_open(O_RDWR)?;
        self.inner
            .lock()
            .file_truncate(size as _)
            .map_err(map_ext4_err)?;
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

    fn unlink(&self, name: &str) -> VfsResult<()> {
        let fpath = self.path_deal_with(name);
        let mut file = self.inner.lock();
        if file.check_inode_exist(&fpath, InodeTypes::EXT4_DE_DIR) {
            // Recursive directory remove
            file.dir_rm(&fpath)
        } else {
            file.file_remove(&fpath)
        }
        .map_err(map_ext4_err)?;
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
        }; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = self.inner.lock().file_size();
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        stat.atime.nsec = 0;
        stat.atime.sec = 0;
        stat.ctime.nsec = 0;
        stat.ctime.sec = 0;
        stat.mtime.nsec = 0;
        stat.mtime.sec = 0;
        Ok(())
        // Err(vfscore::VfsError::NotSupported)
    }

    fn statfs(&self, statfs: &mut StatFS) -> VfsResult<()> {
        statfs.ftype = 32;
        statfs.bsize = 512;
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
        log::warn!("not support utimes for utimes now");
        // Err(vfscore::VfsError::NotSupported)
        Ok(())
    }
}
