#![no_std]
#![feature(let_chains)]

#[allow(unused_imports)]
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;
extern crate bitflags;

pub mod dentry;
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
};
use dentry::mount_fs;
use devfs::{DevDir, DevFS};
use devices::get_blk_devices;
use procfs::ProcFS;
use ramfs::RamFs;
use syscalls::Errno;
use vfscore::VfsResult;
pub use vfscore::{FileType, INodeInterface, SeekFrom};

pub fn build_devfs() -> Arc<DevFS> {
    let dev_dir = DevDir::new();

    DevFS::new_with_dir(dev_dir)
}

pub fn init() {
    info!("fs module initialized");
    // TODO: Identify the filesystem at the device.
    if !get_blk_devices().is_empty() {
        #[cfg(root_fs = "fat32")]
        mount_fs(fatfs_shim::Fat32FileSystem::new(0), "/");
        #[cfg(root_fs = "ext4")]
        mount_fs(ext4fs::Ext4FileSystem::new(0), "/");
        #[cfg(root_fs = "ext4_rs")]
        mount_fs(ext4rsfs::Ext4FileSystem::new(0), "/");
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
}

pub struct WaitBlockingRead<'a>(pub Arc<dyn INodeInterface>, pub &'a mut [u8], pub usize);

impl Future for WaitBlockingRead<'_> {
    type Output = VfsResult<usize>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let offset = self.2;
        let file = self.0.clone();
        let buffer = &mut self.1;
        match file.readat(offset, buffer) {
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

impl Future for WaitBlockingWrite<'_> {
    type Output = VfsResult<usize>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let offset = self.2;
        let file = self.0.clone();
        let buffer = &self.1;

        match file.writeat(offset, buffer) {
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
