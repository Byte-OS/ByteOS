use alloc::{collections::BTreeMap, string::String, sync::Arc};
use log::warn;
use sync::Mutex;
use vfscore::{FileType, INodeInterface, MountedInfo, OpenFlags, VfsError, VfsResult};

use crate::{File, FILESYSTEMS};

pub static MOUNTS: Mutex<BTreeMap<String, MountedInfo>> = Mutex::new(BTreeMap::new());

pub fn init() {
    for (i, fs) in FILESYSTEMS.iter().enumerate() {
        match fs.name() {
            "fat32" => mount(String::from("/"), i).expect("can't mount to /"),
            "ramfs" => mount(String::from("/tmp"), i).expect("can't mount to /ramfs"),
            "devfs" => mount(String::from("/dev"), i).expect("can't mount to /dev"),
            "procfs" => mount(String::from("/proc"), i).expect("can't mount to /procfs"),
            fs => warn!("unsupport fs: {}", fs),
        };
    }

    println!("{:=^30}", " LIST FILES START ");
    list_files(open("/").expect("can't find mount point at ."), 0);
    println!("{:=^30}", " LIST FILES START ");
}

fn list_files(file: File, space: usize) {
    for i in file.read_dir().expect("can't read dir") {
        println!("{:<3$}{} {}", "", i.filename, i.len, space);
        if i.file_type == FileType::Directory {
            list_files(
                file.open(&i.filename, OpenFlags::O_RDWR)
                    .expect("can't read dir"),
                space + 4,
            );
        }
    }
}

pub fn mount(path: String, fs_id: usize) -> VfsResult<()> {
    if path != "/" {
        // judge whether the mount point exists
        open(&path)?;
    }
    MOUNTS.lock().insert(
        path.clone(),
        MountedInfo {
            fs_id,
            path: Arc::new(path),
        },
    );
    Ok(())
}

pub fn umount(path: &str) -> VfsResult<()> {
    MOUNTS.lock().remove(path);
    Ok(())
}

pub fn open(path: &str) -> VfsResult<Arc<dyn INodeInterface>> {
    let mps = MOUNTS.lock().clone();
    for (mount_point, mi) in mps.iter().rev() {
        if path.starts_with(mount_point) {
            let folder = FILESYSTEMS[mi.fs_id].root_dir(mi.clone());
            return path[mount_point.len()..]
                .trim()
                .split('/')
                .fold(Ok(folder), |folder, x| {
                    if x == "." || x == "" {
                        return folder;
                    }
                    folder?.open(x, OpenFlags::O_RDWR)
                });
        }
    }
    Err(VfsError::FileNotFound)
}

pub fn open_mount(path: &str) -> Option<Arc<dyn INodeInterface>> {
    let mps = MOUNTS.lock().clone();
    for (mount_point, mi) in mps.iter().rev() {
        if mount_point == path {
            return Some(FILESYSTEMS[mi.fs_id].root_dir(mi.clone()));
        }
    }
    None
}
