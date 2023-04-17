use core::cmp::min;

use alloc::string::String;
use alloc::sync::Arc;
use devices::get_blk_device;
use fatfs::{Dir, Error, File, LossyOemCpConverter, NullTimeProvider};
use fatfs::{Read, Seek, SeekFrom, Write};
use log::debug;
use sync::Mutex;
use vfscore::{
    DirEntry, FileSystem, FileType, INodeInterface, Metadata, MountedInfo, Stat, VfsError,
    VfsResult,
};

use crate::mount::open_mount;

pub trait DiskOperation {
    fn read_block(index: usize, buf: &mut [u8]);
    fn write_block(index: usize, data: &[u8]);
}

pub struct Fat32FileSystem {
    inner: fatfs::FileSystem<DiskCursor, NullTimeProvider, LossyOemCpConverter>,
}

unsafe impl Send for Fat32FileSystem {}

unsafe impl Sync for Fat32FileSystem {}

impl FileSystem for Fat32FileSystem {
    fn name(&self) -> &str {
        "fat32"
    }

    fn root_dir(&'static self, mi: MountedInfo) -> Arc<dyn INodeInterface> {
        Arc::new(FatDir {
            filename: String::from(""),
            fs: mi.clone(),
            inner: self.inner.root_dir(),
            dir_path: String::from(""),
        })
    }

    fn flush(&self) -> VfsResult<()> {
        self.inner.flush_fs_info().map_err(as_vfs_err)
    }
}

impl Fat32FileSystem {
    pub fn new(device_id: usize) -> Arc<Self> {
        let cursor: DiskCursor = DiskCursor {
            sector: 0,
            offset: 0,
            device_id,
        };
        let inner = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new()).expect("open fs wrong");
        Arc::new(Self { inner })
    }
}

pub struct FatFileInner {
    offset: usize,
    inner: File<'static, DiskCursor, NullTimeProvider, LossyOemCpConverter>,
}

#[allow(dead_code)]
pub struct FatFile {
    filename: String,
    dir_path: String,
    fs: MountedInfo,
    inner: Mutex<FatFileInner>,
}

// TODO: impl Sync and send in safe way
unsafe impl Sync for FatFile {}
unsafe impl Send for FatFile {}

pub struct FatDir {
    filename: String,
    dir_path: String,
    fs: MountedInfo,
    inner: Dir<'static, DiskCursor, NullTimeProvider, LossyOemCpConverter>,
}

// TODO: impl Sync and send in safe way
unsafe impl Sync for FatDir {}
unsafe impl Send for FatDir {}

impl INodeInterface for FatFile {
    fn read(&self, buffer: &mut [u8]) -> VfsResult<usize> {
        let mut inner = self.inner.lock();
        let offset = inner.offset;
        let len = inner.inner.seek(SeekFrom::End(0)).map_err(as_vfs_err)?;
        inner
            .inner
            .seek(SeekFrom::Start(offset as u64))
            .map_err(as_vfs_err)?;
        let rlen = min(buffer.len(), len as usize - inner.offset);
        inner
            .inner
            .read_exact(&mut buffer[..rlen])
            .map_err(as_vfs_err)?;
        inner.offset += rlen;
        Ok(rlen)
    }

    fn write(&self, buffer: &[u8]) -> VfsResult<usize> {
        let mut inner = self.inner.lock();
        inner.inner.write_all(buffer).map_err(as_vfs_err)?;
        inner.offset += buffer.len();
        Ok(buffer.len())
    }

    fn flush(&self) -> VfsResult<()> {
        self.inner.lock().inner.flush().map_err(as_vfs_err)
        // self.fs.upgrade().unwrap().flush()
    }

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        let mut inner = self.inner.lock();
        let len = inner.inner.seek(SeekFrom::End(0)).map_err(as_vfs_err)?;
        let offset = inner.offset as u64;
        inner
            .inner
            .seek(SeekFrom::Start(offset))
            .map_err(as_vfs_err)?;

        Ok(vfscore::Metadata {
            filename: &self.filename,
            inode: usize::MAX,
            file_type: FileType::File,
            size: len as usize,
            childrens: usize::MAX,
        })
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        self.inner
            .lock()
            .inner
            .seek(SeekFrom::Start(size as u64))
            .map_err(as_vfs_err)?;
        self.inner.lock().inner.truncate().map_err(as_vfs_err)
    }

    fn path(&self) -> VfsResult<String> {
        let mount_path = self.fs.path.as_ref().clone();
        Ok(format!("{}{}/{}", mount_path, self.dir_path, self.filename))
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.dev = self.fs.fs_id as u64;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = 0o777; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 1000;
        stat.gid = 1000;
        stat.size = self.metadata().unwrap().size as u64;
        stat.blksize = 512;
        stat.blocks = self.metadata().unwrap().size as u64 / 512;
        stat.rdev = 0; // TODO: add device id
                       // TODO: add A/M/C time
        Ok(())
    }

    fn seek(&self, seek: vfscore::SeekFrom) -> VfsResult<usize> {
        let mut inner = self.inner.lock();
        inner.inner
            .seek(match seek {
                vfscore::SeekFrom::SET(offset) => SeekFrom::Start(offset as u64),
                vfscore::SeekFrom::CURRENT(offset) => SeekFrom::Current(offset as i64),
                vfscore::SeekFrom::END(offset) => SeekFrom::End(offset as i64),
            })
            .map_or_else(|e| Err(as_vfs_err(e)), |x| {
                inner.offset = x as usize;
                Ok(x as usize)
            })
    }
}

impl INodeInterface for FatDir {
    fn mkdir(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .create_dir(name)
            .map(|dir| -> Arc<dyn INodeInterface> {
                Arc::new(FatDir {
                    dir_path: format!("{}/{}", self.dir_path, self.filename),
                    filename: String::from(name),
                    fs: self.fs.clone(),
                    inner: dir,
                })
            })
            .map_err(as_vfs_err)
    }

    fn touch(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        debug!("touch file {} @ {}/{}", name, self.dir_path, self.filename);
        self.inner
            .create_file(name)
            .map(|file| -> Arc<dyn INodeInterface> {
                Arc::new(FatFile {
                    dir_path: format!("{}/{}", self.dir_path, self.filename),
                    filename: String::from(name),
                    fs: self.fs.clone(),
                    inner: Mutex::new(FatFileInner {
                        inner: file,
                        offset: 0,
                    }),
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
            let mut mount_path = self.fs.path.as_ref().clone();
            if mount_path == "/" {
                mount_path = String::from("");
            }
            let res = match open_mount(&format!("{}{}/{}", mount_path, self.dir_path, name)) {
                Some(inode) => inode,
                None => Arc::new(FatDir {
                    dir_path: format!("{}/{}", self.dir_path, self.filename),
                    filename: String::from(name),
                    fs: self.fs.clone(),
                    inner: file.to_dir(),
                }),
            };

            return Ok(res);
            // This is a temporary method for supporting mount
            // 1. create a new dir as mount point
            // 2. touch a .mount file with the target path at the folder
            // use open to open mount point
            // if dir.iter().count() == 0 {

            // }
            // return match dir.open_file(".mount") {
            //     Ok(mut file) => {
            //         let len = file.seek(SeekFrom::End(0)).unwrap_or(0);
            //         file.seek(SeekFrom::Start(0)).expect("can't seek");
            //         let mut buf = vec![0u8; len as usize];
            //         file.read_exact(&mut buf).expect("can't read file");
            //         let mount_point = String::from_utf8(buf).expect("can't conver path");
            //         open(&mount_point)
            //     }
            //     Err(_) => Ok(Arc::new(FatDir {
            //         dir_path: format!("{}/{}", self.dir_path, name),
            //         filename: String::from(name),
            //         fs: self.fs.clone(),
            //         inner: file.to_dir(),
            //     })),
            // };
        };

        if file.is_file() {
            return Ok(Arc::new(FatFile {
                dir_path: format!("{}/{}", self.dir_path, self.filename),
                filename: String::from(name),
                fs: self.fs.clone(),
                inner: Mutex::new(FatFileInner {
                    inner: file.to_file(),
                    offset: 0,
                }),
            }));
        }

        unreachable!()
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        self.inner.remove(name).map_err(as_vfs_err)
    }

    fn remove(&self, name: &str) -> VfsResult<()> {
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

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        Ok(Metadata {
            filename: &self.filename,
            inode: usize::MAX,
            file_type: FileType::Directory,
            size: 0,
            childrens: self.inner.iter().count(),
        })
    }

    fn path(&self) -> VfsResult<String> {
        let mount_path = self.fs.path.as_ref().clone();
        Ok(format!("{}{}/{}", mount_path, self.dir_path, self.filename))
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.dev = self.fs.fs_id as u64;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = 0o777; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 1000;
        stat.gid = 1000;
        stat.size = 0;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        Ok(())
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
