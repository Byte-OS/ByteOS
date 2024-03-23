#![no_std]

extern crate alloc;
extern crate log;

use alloc::{collections::BTreeMap, string::ToString, sync::Arc, vec::Vec};
use vfscore::{DirEntry, FileSystem, FileType, INodeInterface, StatMode, VfsError, VfsResult};

mod cpu_dma_latency;
mod null;
mod rtc;
mod sdx;
mod shm;
mod tty;
mod urandom;
mod zero;

pub use {sdx::Sdx, tty::Tty};

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
    fn root_dir(&self) -> Arc<dyn INodeInterface> {
        Arc::new(DevDirContainer {
            inner: self.root_dir.clone(),
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
}

impl DevDir {
    pub fn new() -> Self {
        let mut map: BTreeMap<&'static str, Arc<dyn INodeInterface>> = BTreeMap::new();
        map.insert("stdout", Arc::new(Tty::new()));
        map.insert("stderr", Arc::new(Tty::new()));
        map.insert("stdin", Arc::new(Tty::new()));
        map.insert("ttyv0", Arc::new(Tty::new()));
        map.insert("null", Arc::new(null::Null));
        map.insert("zero", Arc::new(zero::Zero));
        map.insert("shm", Arc::new(shm::Shm));
        map.insert("rtc", Arc::new(rtc::Rtc));
        map.insert("urandom", Arc::new(urandom::Urandom));
        map.insert("cpu_dma_latency", Arc::new(cpu_dma_latency::CpuDmaLatency));
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
}
