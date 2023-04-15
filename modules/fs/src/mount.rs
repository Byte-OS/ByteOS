use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use log::{warn, debug};
use sync::Mutex;
use vfscore::{INodeInterface, MountedInfo, OpenFlags, VfsError, VfsResult};

use crate::FILESYSTEMS;

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
    let path = String::from("/") + &path.split("/").fold(Vec::new(), |mut vec, x| {
        match x {
            "" | "." => {},
            ".." => {
                if vec.len() > 0 {
                    vec.pop();
                }
            }
            _ => vec.push(x)
        }
        vec
    }).join("/");
    
    debug!("open @ {}", path);
    
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
