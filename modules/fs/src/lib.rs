#![no_std]
#![feature(extract_if)]
#![feature(associated_type_bounds)]
#![feature(let_chains)]

use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    usize,
};

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use devfs::{DevDir, DevFS, Sdx};
use devices::get_blk_devices;
use procfs::ProcFS;
use ramfs::RamFs;
use sync::LazyInit;
use vfscore::{FileSystem, VfsResult};

use crate::{
    dentry::{dentry_init, DentryNode},
    fatfs_shim::Fat32FileSystem,
};

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate logging;

pub mod dentry;
mod fatfs_shim;
pub mod pipe;

pub type File = Arc<dyn INodeInterface>;

pub use vfscore::{
    FileType, INodeInterface, OpenFlags, PollEvent, PollFd, SeekFrom, Stat, StatFS, StatMode,
    TimeSpec, VfsError, UTIME_NOW, UTIME_OMIT,
};
pub static FILESYSTEMS: LazyInit<Vec<Arc<dyn FileSystem>>> = LazyInit::new();

pub fn build_devfs(filesystems: &Vec<(Arc<dyn FileSystem>, &str)>) -> Arc<DevFS> {
    let dev_sdxs: Vec<_> = filesystems
        .iter()
        .enumerate()
        .map(|(i, _x)| {
            Arc::new(Sdx::new(
                i,
                |fs_id, path| {
                    DentryNode::mount(String::from(path), get_filesystem(fs_id).root_dir())
                },
                |_fs_id, path| DentryNode::unmount(String::from(path)),
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

    // TODO: Identify the filesystem at the device.
    let mut filesystems: Vec<(Arc<dyn FileSystem>, &str)> = Vec::new();
    if get_blk_devices().len() > 0 {
        filesystems.push((Fat32FileSystem::new(0), "/"));
    } else {
        filesystems.push((RamFs::new(), "/"));
    }
    filesystems.push((build_devfs(&filesystems), "/dev"));
    filesystems.push((RamFs::new(), "/tmp"));
    filesystems.push((RamFs::new(), "/dev/shm"));
    filesystems.push((RamFs::new(), "/home"));
    filesystems.push((RamFs::new(), "/var"));
    filesystems.push((ProcFS::new(), "/proc"));
    // filesystems.push((RamFs::new(), "/bin"));

    // mount to FILESYSTEMS
    FILESYSTEMS.init_by(filesystems.iter().map(|(fs, _)| fs.clone()).collect());

    // init mount points
    info!("create fatfs mount file");
    {
        // create monnt point dev, tmp
        // let fs = &filesystems[0].0;
        // let rootfs = filesystems[0].0.root_dir();
        let rootfs = get_filesystem(0).root_dir();
        let dev = rootfs.mkdir("dev").expect("can't create devfs dir");
        dev.mkdir("shm").expect("can't create shm dir");
        rootfs.mkdir("tmp").expect("can't create tmp dir");
        // rootfs.mkdir("lib").expect("can't create lib dir");
        rootfs.mkdir("home").expect("can't create home dir");
        rootfs.mkdir("var").expect("can't create var dir");
        rootfs.mkdir("proc").expect("can't create proc dir");
        rootfs.mkdir("bin").expect("can't create var dir");
    }
    for (i, (_, mount_point)) in filesystems.iter().enumerate() {
        // mount(mount_point.to_string(), i).expect(&format!("can't mount fs_{i} {mount_point}"));
        if *mount_point == "/" {
            dentry_init(get_filesystem(i).root_dir())
        } else {
            DentryNode::mount(mount_point.to_string(), get_filesystem(i).root_dir())
                .expect(&format!("can't mount fs_{i} {mount_point}"));
        }
    }
}

pub fn get_filesystem(id: usize) -> &'static Arc<dyn FileSystem> {
    &FILESYSTEMS[id]
}

pub struct WaitBlockingRead<'a>(pub Arc<dyn INodeInterface>, pub &'a mut [u8], pub usize);

impl<'a> Future for WaitBlockingRead<'a> {
    type Output = VfsResult<usize>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let offset = self.2;
        let file = self.0.clone();
        let buffer = &mut self.1;
        match file.readat(offset, *buffer) {
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

pub struct WaitBlockingWrite<'a>(pub Arc<dyn INodeInterface>, pub &'a [u8], pub usize);

impl<'a> Future for WaitBlockingWrite<'a> {
    type Output = VfsResult<usize>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let offset = self.2;
        let file = self.0.clone();
        let buffer = &self.1;

        match file.writeat(offset, *buffer) {
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
