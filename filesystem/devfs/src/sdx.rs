use alloc::{string::String, vec::Vec};
use libc_types::types::{Stat, StatMode};
use sync::Mutex;
use syscalls::Errno;
use vfscore::INodeInterface;

pub struct Sdx {
    device_id: usize,
    mount_fn: fn(usize, &str) -> Result<(), Errno>,
    umount_fn: fn(usize, &str) -> Result<(), Errno>,
    mount_paths: Mutex<Vec<String>>,
}

impl Sdx {
    pub fn new(
        device_id: usize,
        mount_fn: fn(usize, &str) -> Result<(), Errno>,
        umount_fn: fn(usize, &str) -> Result<(), Errno>,
    ) -> Self {
        Self {
            device_id,
            mount_fn,
            umount_fn,
            mount_paths: Mutex::new(Vec::new()),
        }
    }
}

impl INodeInterface for Sdx {
    fn mount(&self, path: &str) -> vfscore::VfsResult<()> {
        let f = self.mount_fn;
        self.mount_paths.lock().push(String::from(path));
        f(self.device_id, path)
    }

    fn umount(&self) -> vfscore::VfsResult<()> {
        let f = self.umount_fn;
        let path = self.mount_paths.lock().pop();
        match path {
            Some(path) => f(self.device_id, &path),
            None => todo!(),
        }
    }

    fn stat(&self, stat: &mut Stat) -> vfscore::VfsResult<()> {
        stat.dev = 0;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::CHAR; // TODO: add access mode
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
