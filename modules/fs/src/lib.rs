#![no_std]
#![feature(drain_filter)]
#![feature(associated_type_bounds)]

use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    usize,
};

use alloc::{string::String, sync::Arc, vec::Vec};
use devfs::{DevDir, DevFS, Sdx};
use devices::get_blk_devices;
use mount::umount;
use ramfs::RamFs;
use sync::LazyInit;
use vfscore::{DirEntry, FileSystem, MountedInfo, VfsResult};

use crate::{
    fatfs_shim::Fat32FileSystem,
    mount::{mount, open},
};

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate logging;

mod cache;
mod fatfs_shim;
pub mod mount;
pub mod pipe;
pub mod socket;

pub type File = Arc<dyn INodeInterface>;

pub use vfscore::{
    FileType, INodeInterface, OpenFlags, PollEvent, PollFd, SeekFrom, Stat, StatFS, StatMode,
    TimeSpec, VfsError, UTIME_NOW, UTIME_OMIT,
};
pub static FILESYSTEMS: LazyInit<Vec<Arc<dyn FileSystem>>> = LazyInit::new();

pub fn build_devfs(filesystems: &Vec<Arc<dyn FileSystem>>) -> Arc<DevFS> {
    let dev_sdxs: Vec<_> = filesystems
        .iter()
        .enumerate()
        .map(|(i, _x)| {
            Arc::new(Sdx::new(
                i,
                |fs_id, path| mount(String::from(path), fs_id),
                |_fs_id, path| umount(path),
            ))
        })
        .collect();
    let mut dev_dir = DevDir::new();

    // TODO: add fs normal, not fixed.
    dev_dir.add("sda", dev_sdxs[0].clone());

    DevFS::new_with_dir(dev_dir)
}

pub fn init() {
    info!("fs module initialized");

    // assert!(get_blk_devices().len() > 0);

    // TODO: Identify the filesystem at the device.
    let mut filesystems: Vec<Arc<dyn FileSystem>> = Vec::new();
    if get_blk_devices().len() > 0 {
        filesystems.push(Fat32FileSystem::new(0));
    } else {
        filesystems.push(RamFs::new());
    }

    // filesystems.push(DevFS::new());
    filesystems.push(build_devfs(&filesystems));
    filesystems.push(RamFs::new());
    filesystems.push(RamFs::new());
    filesystems.push(RamFs::new());

    FILESYSTEMS.init_by(filesystems);

    // init mount points
    info!("create fatfs mount file");
    {
        // create monnt point dev, tmp
        let rootfs = get_filesystem(0).root_dir(MountedInfo::default());
        rootfs.mkdir("dev").expect("can't create devfs dir");
        rootfs.mkdir("tmp").expect("can't create devfs dir");
        rootfs.mkdir("lib").expect("can't create devfs dir");
        rootfs.mkdir("tmp_home").expect("can't create devfs dir");

        let so_files: Vec<DirEntry> = rootfs
            .read_dir()
            .expect("can't read files")
            .into_iter()
            .filter(|x| x.filename.ends_with("dso.so"))
            .collect();

        for file in so_files {
            rootfs
                .link(&file.filename[3..], &format!("/{}", file.filename))
                .expect("can't link file");
        }
    }

    mount::init();
    {
        let rootfs = get_filesystem(0).root_dir(MountedInfo::default());
        let tmpfs = open("/tmp_home").expect("can't open /tmp_home");
        for file in rootfs.read_dir().expect("can't read files") {
            tmpfs
                .link(&file.filename, &(String::from("/") + &file.filename))
                .expect("can't link file to tmpfs");
        }
    }
    cache::init();
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
            }
        }
    }
}

pub struct WaitBlockingWrite<'a>(pub Arc<dyn INodeInterface>, pub &'a [u8]);

impl<'a> Future for WaitBlockingWrite<'a> {
    type Output = VfsResult<usize>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let file = self.0.clone();
        let buffer = &self.1;
        match file.write(*buffer) {
            Ok(wsize) => Poll::Ready(Ok(wsize)),
            Err(err) => {
                if let VfsError::Blocking = err {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            }
        }
    }
}
