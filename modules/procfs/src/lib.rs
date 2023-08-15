#![no_std]

extern crate alloc;

mod meminfo;
mod mounts;

use core::mem::size_of;

use alloc::{
    collections::BTreeMap,
    string::ToString,
    sync::Arc,
    vec::Vec,
};
use meminfo::MemInfo;
use mounts::Mounts;
use sync::Mutex;
use vfscore::{
    DirEntry, Dirent64, FileSystem, FileType, INodeInterface, StatMode, VfsError,
    VfsResult,
};

pub struct ProcFS {
    root: Arc<ProcDir>,
}

impl ProcFS {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            root: ProcDir::new(),
        })
    }
}

impl FileSystem for ProcFS {
    fn root_dir(&'static self) -> Arc<dyn INodeInterface> {
        Arc::new(DevDirContainer {
            inner: self.root.clone(),
            dents_off: Mutex::new(0),
        })
    }

    fn name(&self) -> &str {
        "procfs"
    }
}

pub struct ProcDir {
    map: BTreeMap<&'static str, Arc<dyn INodeInterface>>,
}

impl ProcDir {
    pub fn new() -> Arc<ProcDir> {
        let mut map: BTreeMap<&str, Arc<dyn INodeInterface>> = BTreeMap::new();
        map.insert("mounts", Arc::new(Mounts::new()));
        map.insert("meminfo", Arc::new(MemInfo::new()));
        Arc::new(ProcDir { map })
    }
}

pub struct DevDirContainer {
    inner: Arc<ProcDir>,
    dents_off: Mutex<usize>,
}

impl INodeInterface for DevDirContainer {
    fn open(&self, name: &str, _flags: vfscore::OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .map
            .get(name)
            .map(|x| x.clone())
            .ok_or(VfsError::FileNotFound)
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Ok(self
            .inner
            .map
            .iter()
            .map(|(name, _)| DirEntry {
                filename: name.to_string(),
                len: 0,
                file_type: FileType::Device,
            })
            .collect())
    }

    fn stat(&self, stat: &mut vfscore::Stat) -> VfsResult<()> {
        stat.dev = 0;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::DIR; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        Ok(())
    }

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        Ok(vfscore::Metadata {
            filename: "dev",
            inode: 0,
            file_type: FileType::Directory,
            size: 0,
            childrens: self.inner.map.len(),
        })
    }

    fn getdents(&self, buffer: &mut [u8]) -> VfsResult<usize> {
        let buf_ptr = buffer.as_mut_ptr() as usize;
        let len = buffer.len();
        let mut ptr: usize = buf_ptr;
        let mut finished = 0;
        for (i, x) in self
            .inner
            .map
            .iter()
            .enumerate()
            .skip(*self.dents_off.lock())
        {
            let filename = x.0;
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
}
