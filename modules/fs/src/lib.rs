#![no_std]

use alloc::{sync::Arc, vec::Vec};
use devfs::DevFS;
use devices::get_blk_devices;
use ramfs::RamFs;
use sync::LazyInit;
use vfscore::{FileSystem, INodeInterface, MountedInfo};

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
    filesystems.push(Fat32FileSystem::new(0));
    filesystems.push(DevFS::new());
    filesystems.push(RamFs::new());

    FILESYSTEMS.init_by(filesystems);

    // init mount points
    info!("create fatfs mount file");
    {
        // create monnt point dev, tmp
        let fatfs = get_filesystem(0).root_dir(MountedInfo::default());
        fatfs.mkdir("dev").expect("can't create devfs dir");
        fatfs.mkdir("tmp").expect("can't create devfs dir");

        // create tets file in ramfs
        get_filesystem(2)
            .root_dir(MountedInfo::default())
            .touch("newfile.txt")
            .expect("can't create file in ramfs")
            .write(b"test data")
            .expect("can't create file in ramfs/newfile.txt");
    }

    mount::init();
}

pub fn get_filesystem(id: usize) -> &'static Arc<dyn FileSystem> {
    &FILESYSTEMS[id]
}
