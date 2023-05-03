use alloc::{string::String, sync::Arc};
use vfscore::{INodeInterface, OpenFlags, StatMode, VfsError::FileNotFound, VfsResult};

pub struct Shm;

impl INodeInterface for Shm {
    fn open(&self, name: &str, flags: OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        extern "Rust" {
            pub fn open_mount(path: &str) -> Option<Arc<dyn INodeInterface>>;
        }
        match unsafe { open_mount("/dev/shm") } {
            Some(file) => file.open(name, flags),
            None => Err(FileNotFound),
        }
    }

    fn path(&self) -> VfsResult<String> {
        Ok(String::from("/dev/shm"))
    }

    fn touch(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        extern "Rust" {
            pub fn open_mount(path: &str) -> Option<Arc<dyn INodeInterface>>;
        }
        match unsafe { open_mount("/dev/shm") } {
            Some(file) => file.touch(name),
            None => Err(vfscore::VfsError::NoMountedPoint),
        }
    }

    fn stat(&self, stat: &mut vfscore::Stat) -> vfscore::VfsResult<()> {
        stat.dev = 0;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::DIR; // TODO: add access mode
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
