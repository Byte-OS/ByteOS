#![no_std]

use alloc::{sync::Arc, vec::Vec};
use devfs::DevFS;
use devices::get_blk_devices;
use sync::LazyInit;
use vfscore::{FileSystem, INodeInterface};

use crate::fatfs_shim::Fat32FileSystem;

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate logging;

mod fatfs_shim;
pub mod mount;

pub type File = Arc<dyn INodeInterface>;
pub use vfscore::{FileType, OpenFlags};
pub static FILESYSTEMS: LazyInit<Vec<Arc<dyn FileSystem>>> = LazyInit::new();

pub fn init() {
    info!("fs module initialized");

    assert!(get_blk_devices().len() > 0);
    // TODO: Identify the filesystem at the device.
    let mut filesystems: Vec<Arc<dyn FileSystem>> = Vec::new();
    filesystems.push(Fat32FileSystem::new(0, filesystems.len()));
    filesystems.push(DevFS::new());

    FILESYSTEMS.init_by(filesystems);

    // init mount points
    info!("create fatfs mount file");
    {
        let dir = get_filesystem(0)
            .root_dir()
            .mkdir("dev")
            .expect("can't create devfs dir");
        let file = dir.touch(".mount").expect("can't create .mount file");
        file.write(b"/dev").expect("can't write file");
    }

    mount::init();
}

pub fn get_filesystem(id: usize) -> &'static Arc<dyn FileSystem> {
    &FILESYSTEMS[id]
}
