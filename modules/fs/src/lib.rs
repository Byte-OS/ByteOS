#![no_std]

use core::{future::Future, pin::Pin, task::{Context, Poll}, usize};

use alloc::{sync::Arc, vec::Vec};
use devfs::DevFS;
use devices::get_blk_devices;
use ramfs::RamFs;
use sync::LazyInit;
use vfscore::{FileSystem, INodeInterface, MountedInfo, VfsResult};

use crate::fatfs_shim::Fat32FileSystem;

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate logging;

mod fatfs_shim;
pub mod mount;
pub mod pipe;

pub type File = Arc<dyn INodeInterface>;
pub use vfscore::{FileType, OpenFlags, Stat, TimeSepc, VfsError};
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

pub struct WaitBlockingRead<'a>(pub Arc<dyn INodeInterface>, pub &'a mut [u8]);

impl<'a> Future for WaitBlockingRead<'a> {
    type Output = VfsResult<usize>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let file = self.0.clone();
        let buffer = &mut self.1;
        match file.read(*buffer) {
            Ok(rsize) => Poll::Ready(Ok(rsize)),
            Err(err) => {
                if let VfsError::Blocking = err {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            },
        }
    }
}
