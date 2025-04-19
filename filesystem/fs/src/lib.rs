#![no_std]
#![feature(let_chains)]

#[allow(unused_imports)]
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;
extern crate bitflags;

pub mod dentry;
#[cfg(root_fs = "ext4_rs")]
mod ext4_rs_shim;
#[cfg(root_fs = "ext4")]
mod ext4_shim;
#[cfg(root_fs = "fat32")]
mod fatfs_shim;
pub mod file;
pub mod pathbuf;
pub mod pipe;

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    usize,
};
use dentry::mount_fs;
use devfs::{DevDir, DevFS};
use devices::get_blk_devices;
use file::File;
use pathbuf::PathBuf;
use procfs::ProcFS;
use ramfs::RamFs;
use syscalls::Errno;
use vfscore::VfsResult;
pub use vfscore::{
    FileType, INodeInterface, OpenFlags, PollEvent, PollFd, SeekFrom, Stat, StatFS, StatMode,
    TimeSpec, UTIME_NOW, UTIME_OMIT,
};

pub fn build_devfs() -> Arc<DevFS> {
    let dev_dir = DevDir::new();

    DevFS::new_with_dir(dev_dir)
}

pub fn init() {
    info!("fs module initialized");
    // TODO: Identify the filesystem at the device.
    if get_blk_devices().len() > 0 {
        #[cfg(root_fs = "fat32")]
        mount_fs(fatfs_shim::Fat32FileSystem::new(0), "/");
        #[cfg(root_fs = "ext4")]
        mount_fs(ext4_shim::Ext4FileSystem::new(0), "/");
        #[cfg(root_fs = "ext4_rs")]
        mount_fs(ext4_rs_shim::Ext4FileSystem::new(0), "/");
    } else {
        mount_fs(RamFs::new(), "/");
    }
    mount_fs(build_devfs(), "/dev");
    mount_fs(RamFs::new(), "/tmp");
    mount_fs(RamFs::new(), "/dev/shm");
    mount_fs(RamFs::new(), "/home");
    mount_fs(RamFs::new(), "/var");
    mount_fs(ProcFS::new(), "/proc");
    // filesystems.push((RamFs::new(), "/bin"));

    // init mount points
    info!("create fatfs mount file");
    {
        // create monnt point dev, tmp
        // let fs = &filesystems[0].0;
        // let rootfs = filesystems[0].0.root_dir();
        let rootfs = File::open(PathBuf::new(), OpenFlags::O_RDONLY).unwrap();
        rootfs.mkdir("dev").expect("can't create devfs dir");
        // dev.mkdir("shm").expect("can't create shm dir");
        rootfs.mkdir("tmp").expect("can't create tmp dir");
        // rootfs.mkdir("lib").expect("can't create lib dir");
        rootfs.mkdir("home").expect("can't create home dir");
        rootfs.mkdir("var").expect("can't create var dir");
        rootfs.mkdir("proc").expect("can't create proc dir");
        rootfs.mkdir("bin").expect("can't create var dir");
    }
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
                if let Errno::EWOULDBLOCK = err {
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
                if let Errno::EWOULDBLOCK = err {
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            }
        }
    }
}
