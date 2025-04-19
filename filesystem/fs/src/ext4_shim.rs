use alloc::{
    ffi::CString,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::iter::zip;
use devices::get_blk_device;
use lwext4_rust::{
    bindings::{ext4_fsymlink, ext4_readlink, O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY},
    Ext4BlockWrapper, Ext4File, InodeTypes, KernelDevOp,
};
use sync::Mutex;
use syscalls::Errno;
use vfscore::{
    DirEntry, FileSystem, FileType, INodeInterface, StatFS, StatMode, TimeSpec, VfsResult,
};

const BLOCK_SIZE: usize = 0x200;

pub struct Ext4DiskWrapper {
    block_id: usize,
    offset: usize,
    blk_id: usize,
}

impl Ext4DiskWrapper {
    /// Create a new disk.
    pub const fn new(blk_id: usize) -> Self {
        Self {
            block_id: 0,
            offset: 0,
            blk_id,
        }
    }

    /// Get the position of the cursor.
    #[inline]
    pub fn position(&self) -> u64 {
        (self.block_id * BLOCK_SIZE + self.offset) as u64
    }

    /// Set the position of the cursor.
    #[inline]
    pub fn set_position(&mut self, pos: u64) {
        self.block_id = pos as usize / BLOCK_SIZE;
        self.offset = pos as usize % BLOCK_SIZE;
    }
}

impl KernelDevOp for Ext4DiskWrapper {
    type DevType = Self;

    fn write(dev: &mut Self::DevType, buf: &[u8]) -> Result<usize, i32> {
        assert!(dev.offset % BLOCK_SIZE == 0);
        get_blk_device(0)
            .expect("can't find block device")
            .write_blocks(dev.block_id, buf);
        dev.block_id += buf.len() / BLOCK_SIZE;
        Ok(buf.len())
    }

    fn read(dev: &mut Self::DevType, buf: &mut [u8]) -> Result<usize, i32> {
        assert!(dev.offset % BLOCK_SIZE == 0);
        get_blk_device(0)
            .expect("can't find block device")
            .read_blocks(dev.block_id, buf);
        dev.block_id += buf.len() / BLOCK_SIZE;
        Ok(buf.len())
    }

    fn seek(dev: &mut Self::DevType, off: i64, whence: i32) -> Result<i64, i32> {
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

    fn flush(_dev: &mut Self::DevType) -> Result<usize, i32> {
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

#[inline(always)]
fn map_ext4_err(err: i32) -> Errno {
    Errno::new(err)
}

impl FileSystem for Ext4FileSystem {
    fn root_dir(&self) -> Arc<dyn INodeInterface> {
        self.root.clone()
    }

    fn name(&self) -> &str {
        "ext4"
    }
}

pub struct Ext4FileWrapper {
    file_type: FileType,
    inner: Mutex<Ext4File>,
}

impl Ext4FileWrapper {
    fn new(path: &str, types: InodeTypes) -> Self {
        let file: Ext4File = Ext4File::new(path, types);
        let file_type = map_ext4_type(file.get_type());
        Self {
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
}

impl INodeInterface for Ext4FileWrapper {
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
        file.file_open(path, O_RDWR).map_err(map_ext4_err)?;
        file.file_seek(offset as _, 0).map_err(map_ext4_err)?;
        let wsize = file.file_write(buffer).map_err(map_ext4_err)?;
        let _ = file.file_close();
        Ok(wsize)
    }

    fn mkdir(&self, name: &str) -> VfsResult<()> {
        log::warn!("mkdir name: {}", name);
        let fpath = self.path_deal_with(&name);
        self.inner.lock().dir_mk(&fpath).map_err(map_ext4_err)?;
        Ok(())
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        self.unlink(name)
    }

    fn remove(&self, name: &str) -> VfsResult<()> {
        self.unlink(name)
    }

    fn symlink(&self, name: &str, src: &str) -> VfsResult<()> {
        let fpath = self.path_deal_with(name);
        let fpath = fpath.as_str();
        if fpath.is_empty() {
            return Ok(());
        }
        let mut file = Ext4File::new(fpath, InodeTypes::EXT4_DE_SYMLINK);
        if file.check_inode_exist(fpath, InodeTypes::EXT4_DE_SYMLINK) {
            return Err(Errno::EEXIST);
        }
        let c_fpath = CString::new(fpath).unwrap();
        let c_src = CString::new(src).unwrap();
        unsafe {
            Errno::from_ret(ext4_fsymlink(c_src.into_raw(), c_fpath.into_raw()) as _)?;
        }
        Ok(())
    }

    fn resolve_link(&self) -> VfsResult<String> {
        let file = self.inner.lock();
        let path = file.get_path();
        let path = path.to_str().unwrap();
        let mut buffer = [0u8; 100];
        let mut rsize = 0;
        unsafe {
            Errno::from_ret(ext4_readlink(
                path.as_ptr() as _,
                buffer.as_mut_ptr() as _,
                buffer.len() as _,
                &mut rsize,
            ) as _)?;
        }
        let str = String::from_utf8_lossy(&buffer[..rsize]);
        Ok(str.to_string())
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        let iters = self
            .inner
            .lock()
            .lwext4_dir_entries()
            .map_err(map_ext4_err)?;
        let mut ans = Vec::new();
        for (name, file_type) in zip(iters.0, iters.1) {
            ans.push(DirEntry {
                filename: CString::from_vec_with_nul(name)
                    .map_err(|_| Errno::EINVAL)?
                    .to_str()
                    .map_err(|_| Errno::EINVAL)?
                    .to_string(),
                len: 0,
                file_type: map_ext4_type(file_type),
            })
        }
        Ok(ans)
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        let fpath = self.path_deal_with(name);
        let fpath = fpath.as_str();
        if fpath.is_empty() {
            return Ok(Arc::new(Ext4FileWrapper::new("/", InodeTypes::EXT4_DE_DIR)));
        }

        let mut file = self.inner.lock();

        if file.check_inode_exist(&fpath, InodeTypes::EXT4_DE_DIR) {
            Ok(Arc::new(Ext4FileWrapper::new(
                fpath,
                InodeTypes::EXT4_DE_DIR,
            )))
        } else if file.check_inode_exist(&fpath, InodeTypes::EXT4_DE_REG_FILE) {
            Ok(Arc::new(Ext4FileWrapper::new(
                fpath,
                InodeTypes::EXT4_DE_REG_FILE,
            )))
        } else if file.check_inode_exist(&fpath, InodeTypes::EXT4_DE_SYMLINK) {
            Ok(Arc::new(Ext4FileWrapper::new(
                fpath,
                InodeTypes::EXT4_DE_SYMLINK,
            )))
        } else {
            Err(Errno::ENOENT)
        }
    }

    fn create(&self, name: &str, ty: FileType) -> VfsResult<()> {
        let ext4_type = match ty {
            FileType::Directory => InodeTypes::EXT4_DE_DIR,
            FileType::File => InodeTypes::EXT4_DE_REG_FILE,
            _ => unimplemented!(),
        };

        let fpath = self.path_deal_with(name);
        let fpath = fpath.as_str();
        if fpath.is_empty() {
            return Ok(());
        }
        let mut file = self.inner.lock();
        if file.check_inode_exist(fpath, ext4_type.clone()) {
            Ok(())
        } else {
            if ext4_type == InodeTypes::EXT4_DE_DIR {
                file.dir_mk(fpath).map_err(map_ext4_err)?;
            } else {
                file.file_open(fpath, O_WRONLY | O_CREAT | O_TRUNC)
                    .map_err(map_ext4_err)?;
                file.file_close().map_err(map_ext4_err)?;
            }
            Ok(())
        }
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        let mut file = self.inner.lock();
        let path = file.get_path();
        let path = path.to_str().unwrap();
        file.file_open(path, O_RDWR).map_err(map_ext4_err)?;

        file.file_truncate(size as _).map_err(map_ext4_err)?;
        let _ = file.file_close();
        Ok(())
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
        let mut file = self.inner.lock();
        // TODO: 读取其他文件的信息
        if self.file_type == FileType::File {
            let path = file.get_path();
            let path = path.to_str().unwrap();
            file.file_open(path, O_RDONLY).map_err(map_ext4_err)?;
        }

        stat.ino = 1; // TODO: 获取真正的 INode
        stat.mode = match file.get_type() {
            InodeTypes::EXT4_DE_REG_FILE => StatMode::FILE,
            InodeTypes::EXT4_DE_DIR => StatMode::DIR,
            InodeTypes::EXT4_DE_BLKDEV => StatMode::BLOCK,
            InodeTypes::EXT4_DE_SOCK => StatMode::SOCKET,
            InodeTypes::EXT4_DE_SYMLINK => StatMode::LINK,
            _ => unreachable!(),
        };
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = file.file_size();
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0;
        stat.atime.nsec = 0;
        stat.atime.sec = 0;
        stat.ctime.nsec = 0;
        stat.ctime.sec = 0;
        stat.mtime.nsec = 0;
        stat.mtime.sec = 0;

        if self.file_type == FileType::File {
            let _ = file.file_close();
        }
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
