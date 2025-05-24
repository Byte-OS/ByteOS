#![no_std]

extern crate alloc;

mod interrupts;
mod meminfo;
mod mounts;

use alloc::{collections::BTreeMap, string::ToString, sync::Arc, vec::Vec};
use interrupts::Interrupts;
use libc_types::types::{Stat, StatMode};
use meminfo::MemInfo;
use mounts::Mounts;
use syscalls::Errno;
use vfscore::{DirEntry, FileSystem, FileType, INodeInterface, VfsResult};

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
    fn root_dir(&self) -> Arc<dyn INodeInterface> {
        Arc::new(DevDirContainer {
            inner: self.root.clone(),
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
        map.insert("interrupts", Arc::new(Interrupts::new()));
        Arc::new(ProcDir { map })
    }
}

pub struct DevDirContainer {
    inner: Arc<ProcDir>,
}

impl INodeInterface for DevDirContainer {
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .map
            .get(name)
            .map(|x| x.clone())
            .ok_or(Errno::ENOENT)
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

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
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
}
