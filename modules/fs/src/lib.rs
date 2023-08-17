#![no_std]
#![feature(drain_filter)]
#![feature(associated_type_bounds)]

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
use mount::umount;
use procfs::ProcFS;
use ramfs::RamFs;
use sync::LazyInit;
use vfscore::{FileSystem, VfsResult};

use crate::{fatfs_shim::Fat32FileSystem, mount::mount};

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate logging;

mod cache;
pub mod dentry;
mod fatfs_shim;
pub mod mount;
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
    filesystems.push((RamFs::new(), "/tmp_home"));
    filesystems.push((RamFs::new(), "/var"));
    filesystems.push((ProcFS::new(), "/proc"));
    filesystems.push((RamFs::new(), "/bin"));

    // mount to FILESYSTEMS
    FILESYSTEMS.init_by(filesystems.iter().map(|(fs, _)| fs.clone()).collect());

    // init mount points
    info!("create fatfs mount file");
    {
        // create monnt point dev, tmp
        // let fs = &filesystems[0].0;
        // let rootfs = filesystems[0].0.root_dir();
        let rootfs = get_filesystem(0).root_dir();
        rootfs.mkdir("dev").expect("can't create devfs dir");
        rootfs.mkdir("tmp").expect("can't create tmp dir");
        rootfs.mkdir("lib").expect("can't create lib dir");
        rootfs.mkdir("tmp_home").expect("can't create tmp_home dir");
        rootfs.mkdir("var").expect("can't create var dir");
        rootfs.mkdir("proc").expect("can't create proc dir");
        rootfs.mkdir("bin").expect("can't create var dir");
    }
    for (i, (_, mount_point)) in filesystems.iter().enumerate() {
        mount(mount_point.to_string(), i).expect(&format!("can't mount fs_{i} {mount_point}"));
    }
    {
        // let cache_file = vec!["busybox", "entry-static.exe", "runtest.exe"];
        let rootfs = get_filesystem(0).root_dir();
        let tmpfs = mount::open("/tmp_home").expect("can't open /tmp_home");
        for file in rootfs.read_dir().expect("can't read files") {
            tmpfs
                .link(
                    &file.filename,
                    rootfs.open(&file.filename, OpenFlags::NONE).unwrap(),
                )
                .expect("can't link file to tmpfs");
        }

        mount::open("/var")
            .expect("can't open /var")
            .mkdir("tmp")
            .expect("can't create tmp dir");

        mount::open("/bin")
            .expect("can't open /bin")
            .link(
                "sleep",
                mount::open("busybox").expect("not hava busybox file"),
            )
            .expect("can't link busybox to /bin/sleep");
    }
    cache::init();
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
