#![no_std]

extern crate alloc;
extern crate log;

use core::mem::size_of;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use sync::Mutex;
use vfscore::{
    DirEntry, Dirent64, FileSystem, FileType, INodeInterface, MountedInfo, StatMode, VfsError,
    VfsResult,
};

mod null;
mod sdx;
mod stdin;
mod stdout;
mod zero;

pub use {sdx::Sdx, stdin::Stdin, stdout::Stdout};

pub struct DevFS {
    root_dir: Arc<DevDir>,
}

impl DevFS {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            root_dir: Arc::new(DevDir::new()),
        })
    }

    pub fn new_with_dir(dev: DevDir) -> Arc<Self> {
        Arc::new(Self {
            root_dir: Arc::new(dev),
        })
    }
}

impl FileSystem for DevFS {
    fn root_dir(&'static self, _mi: MountedInfo) -> Arc<dyn INodeInterface> {
        Arc::new(DevDirContainer {
            inner: self.root_dir.clone(),
            dents_off: Mutex::new(0),
        })
    }

    fn name(&self) -> &str {
        "devfs"
    }
}

pub struct DevDir {
    map: BTreeMap<&'static str, Arc<dyn INodeInterface>>,
}

pub struct DevDirContainer {
    inner: Arc<DevDir>,
    dents_off: Mutex<usize>,
}

impl DevDir {
    pub fn new() -> Self {
        let mut map: BTreeMap<&'static str, Arc<dyn INodeInterface>> = BTreeMap::new();
        map.insert("stdout", Arc::new(stdout::Stdout));
        map.insert("stderr", Arc::new(stdout::Stdout));
        map.insert("stdin", Arc::new(stdin::Stdin));
        map.insert("null", Arc::new(null::Null));
        map.insert("zero", Arc::new(zero::Zero));
        // map.insert("tty", Arc::new(stdout::Stdout));

        Self { map }
    }

    pub fn add(&mut self, path: &'static str, node: Arc<dyn INodeInterface>) {
        self.map.insert(path, node);
    }
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

    fn path(&self) -> VfsResult<alloc::string::String> {
        Ok(String::from("/dev"))
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
