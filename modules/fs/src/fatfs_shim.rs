use alloc::sync::{Arc, Weak};
use devices::get_blk_device;
use fatfs::{Dir, Error, File, LossyOemCpConverter, NullTimeProvider};
use fatfs::{Read, Seek, SeekFrom, Write};
use vfscore::{DirEntry, FileSystem, FileType, INodeInterface, Metadata, VfsError, VfsResult};

use crate::FILESYSTEMS;

pub trait DiskOperation {
    fn read_block(index: usize, buf: &mut [u8]);
    fn write_block(index: usize, data: &[u8]);
}

pub struct Fat32FileSystem {
    id: usize,
    inner: fatfs::FileSystem<DiskCursor, NullTimeProvider, LossyOemCpConverter>,
}

unsafe impl Send for Fat32FileSystem {}

unsafe impl Sync for Fat32FileSystem {}

impl FileSystem for Fat32FileSystem {
    fn name(&self) -> &str {
        "fat32"
    }

    fn root_dir(&'static self) -> Arc<dyn INodeInterface> {
        Arc::new(FatDir {
            fs: Arc::downgrade(&FILESYSTEMS[self.id]),
            inner: self.inner.root_dir(),
        })
    }
}

impl Fat32FileSystem {
    pub fn new(device_id: usize, id: usize) -> Arc<Self> {
        let cursor: DiskCursor = DiskCursor {
            sector: 0,
            offset: 0,
            device_id,
        };
        let innder =
            fatfs::FileSystem::new(cursor, fatfs::FsOptions::new()).expect("open fs wrong");
        Arc::new(Self { id, inner: innder })
    }
}

pub struct FatFile {
    offset: usize,
    fs: Weak<dyn FileSystem>,
    inner: File<'static, DiskCursor, NullTimeProvider, LossyOemCpConverter>,
}

// TODO: impl Sync and send in safe way
unsafe impl Sync for FatFile {}
unsafe impl Send for FatFile {}

pub struct FatDir {
    fs: Weak<dyn FileSystem>,
    inner: Dir<'static, DiskCursor, NullTimeProvider, LossyOemCpConverter>,
}

// TODO: impl Sync and send in safe way
unsafe impl Sync for FatDir {}
unsafe impl Send for FatDir {}

impl INodeInterface for FatFile {
    fn read(&mut self, buffer: &mut [u8]) -> VfsResult<usize> {
        let len = self.inner.seek(SeekFrom::End(0)).map_err(as_vfs_err)?;
        self.inner
            .seek(SeekFrom::Start(self.offset as u64))
            .map_err(as_vfs_err)?;
        self.inner.read_exact(buffer).map_err(as_vfs_err)?;
        self.offset += len as usize - self.offset;
        Ok(len as usize - self.offset)
    }

    fn write(&mut self, buffer: &[u8]) -> VfsResult<usize> {
        self.inner.write_all(buffer).map_err(as_vfs_err)?;
        self.offset += buffer.len();
        Ok(buffer.len())
    }

    fn weak_filesystem(&self) -> VfsResult<Weak<dyn FileSystem>> {
        Ok(self.fs.clone())
    }

    fn flush(&mut self) -> VfsResult<()> {
        self.inner.flush().map_err(as_vfs_err)
    }

    fn metadata(&mut self) -> VfsResult<vfscore::Metadata> {
        let len = self.inner.seek(SeekFrom::End(0)).map_err(as_vfs_err)?;

        Ok(vfscore::Metadata {
            inode: usize::MAX,
            file_type: FileType::File,
            size: len as usize,
            childrens: usize::MAX,
        })
    }

    fn truncate(&mut self, size: usize) -> VfsResult<()> {
        self.inner
            .seek(SeekFrom::Start(size as u64))
            .map_err(as_vfs_err)?;
        self.inner.truncate().map_err(as_vfs_err)
    }
}

impl INodeInterface for FatDir {
    fn mkdir(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .create_dir(name)
            .map(|dir| -> Arc<dyn INodeInterface> {
                Arc::new(FatDir {
                    fs: self.fs.clone(),
                    inner: dir,
                })
            })
            .map_err(as_vfs_err)
    }

    fn touch(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .create_file(name)
            .map(|file| -> Arc<dyn INodeInterface> {
                Arc::new(FatFile {
                    fs: self.fs.clone(),
                    inner: file,
                    offset: 0,
                })
            })
            .map_err(as_vfs_err)
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        todo!()
    }

    fn open(&self, name: &str, _flags: vfscore::OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        let file = self
            .inner
            .iter()
            .find(|f| f.as_ref().unwrap().file_name() == name);
        let file = file.map(|x| x.unwrap()).ok_or(VfsError::FileNotFound)?;
        if file.is_dir() {
            return Ok(Arc::new(FatDir {
                fs: self.fs.clone(),
                inner: file.to_dir(),
            }));
        };

        if file.is_file() {
            return Ok(Arc::new(FatFile {
                fs: self.fs.clone(),
                offset: 0,
                inner: file.to_file(),
            }));
        }

        unreachable!()
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        self.inner.remove(name).map_err(as_vfs_err)
    }

    fn read_dir(&self) -> VfsResult<alloc::vec::Vec<vfscore::DirEntry>> {
        Ok(self
            .inner
            .iter()
            .filter_map(|x| {
                let x = x.unwrap();
                if x.file_name() == "." || x.file_name() == ".." {
                    return None;
                }
                let file_type = {
                    if x.is_file() {
                        FileType::File
                    } else if x.is_dir() {
                        FileType::Directory
                    } else {
                        unreachable!()
                    }
                };
                Some(DirEntry {
                    filename: x.file_name(),
                    len: x.len() as usize,
                    file_type,
                })
            })
            .collect())
    }

    fn metadata(&mut self) -> VfsResult<vfscore::Metadata> {
        Ok(Metadata {
            inode: usize::MAX,
            file_type: FileType::Directory,
            size: 0,
            childrens: self.inner.iter().count(),
        })
    }
}

pub const fn as_vfs_err(err: Error<()>) -> vfscore::VfsError {
    match err {
        Error::AlreadyExists => VfsError::AlreadyExists,
        Error::CorruptedFileSystem => VfsError::InvalidData,
        Error::DirectoryIsNotEmpty => VfsError::DirectoryNotEmpty,
        Error::InvalidInput
        | Error::InvalidFileNameLength
        | Error::UnsupportedFileNameCharacter => VfsError::InvalidInput,
        Error::NotEnoughSpace => VfsError::StorageFull,
        Error::NotFound => VfsError::FileNotFound,
        Error::UnexpectedEof => VfsError::UnexpectedEof,
        Error::WriteZero => VfsError::WriteZero,
        Error::Io(_) => VfsError::Io,
        _ => VfsError::Io,
    }
}

pub struct DiskCursor {
    sector: u64,
    offset: usize,
    device_id: usize,
}

unsafe impl Sync for DiskCursor {}
unsafe impl Send for DiskCursor {}

impl DiskCursor {
    fn get_position(&self) -> usize {
        (self.sector * 0x200) as usize + self.offset
    }

    fn set_position(&mut self, position: usize) {
        self.sector = (position / 0x200) as u64;
        self.offset = position % 0x200;
    }

    fn move_cursor(&mut self, amount: usize) {
        self.set_position(self.get_position() + amount)
    }
}

impl fatfs::IoBase for DiskCursor {
    type Error = ();
}

impl fatfs::Read for DiskCursor {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // 由于读取扇区内容还需要考虑跨 cluster，因此 read 函数只读取一个扇区
        // 防止读取较多数据时超出限制
        // 读取所有的数据的功能交给 read_exact 来实现

        // 如果 start 不是 0 或者 len 不是 512
        let device = get_blk_device(self.device_id).unwrap();
        let read_size = if self.offset != 0 || buf.len() < 512 {
            let mut data = vec![0u8; 512];
            device.read_block(self.sector as usize, &mut data);

            let start = self.offset;
            let end = (self.offset + buf.len()).min(512);

            buf.copy_from_slice(&data[start..end]);
            end - start
        } else {
            device.read_block(self.sector as usize, &mut buf[0..512]);
            512
        };

        self.move_cursor(read_size);
        Ok(read_size)
    }
}

impl fatfs::Write for DiskCursor {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        // 由于写入扇区还需要考虑申请 cluster，因此 write 函数只写入一个扇区
        // 防止写入较多数据时超出限制
        // 写入所有的数据的功能交给 write_all 来实现

        // 获取硬盘设备写入器（驱动？）
        // 如果 start 不是 0 或者 len 不是 512
        let device = get_blk_device(self.device_id).unwrap();
        let write_size = if self.offset != 0 || buf.len() < 512 {
            let mut data = vec![0u8; 512];
            device.read_block(self.sector as usize, &mut data);

            let start = self.offset;
            let end = (self.offset + buf.len()).min(512);

            data[start..end].clone_from_slice(&buf);
            device.write_block(self.sector as usize, &mut data);

            end - start
        } else {
            device.write_block(self.sector as usize, &buf[0..512]);
            512
        };

        self.move_cursor(write_size);
        Ok(write_size)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl fatfs::Seek for DiskCursor {
    fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, Self::Error> {
        match pos {
            fatfs::SeekFrom::Start(i) => {
                self.set_position(i as usize);
                Ok(i)
            }
            fatfs::SeekFrom::End(_) => unreachable!(),
            fatfs::SeekFrom::Current(i) => {
                let new_pos = (self.get_position() as i64) + i;
                self.set_position(new_pos as usize);
                Ok(new_pos as u64)
            }
        }
    }
}
