use alloc::{vec::Vec, string::String};
use sync::Mutex;
use vfscore::{INodeInterface, VfsError};

pub struct Sdx {
    device_id: usize,
    mount_fn: fn(usize, &str) -> Result<(), VfsError>,
    umount_fn: fn(usize, &str) -> Result<(), VfsError>,
    mount_paths: Mutex<Vec<String>>
}

impl Sdx {
    pub fn new(
        device_id: usize, 
        mount_fn: fn(usize, &str) -> Result<(), VfsError>,
        umount_fn: fn(usize, &str) -> Result<(), VfsError>,
    ) -> Self {
        Self {
            device_id,
            mount_fn,
            umount_fn,
            mount_paths: Mutex::new(Vec::new())
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
}
