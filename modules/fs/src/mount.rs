use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use log::debug;
use sync::Mutex;
use vfscore::{INodeInterface, OpenFlags, VfsError, VfsResult};

use crate::FILESYSTEMS;

pub static MOUNTS: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());

pub fn init() {
    mount(String::from("/"), 0).expect("can't mount to /");
    mount(String::from("/dev"), 1).expect("can't mount to /dev");
    mount(String::from("/tmp"), 2).expect("can't mount to /tmp");
    mount(String::from("/dev/shm"), 3).expect("can't mount to /dev/shm");
    mount(String::from("/tmp_home"), 4).expect("can't mount to /tmp_home");
    mount(String::from("/var"), 5).expect("can't mount to /var");
    mount(String::from("/proc"), 6).expect("can't mount to /proc");
    // mount(String::from("/bin"), 7).expect("can't mount to /bin");
}

pub fn mount(path: String, fs_id: usize) -> VfsResult<()> {
    if path != "/" {
        // judge whether the mount point exists
        open(&path)?;
    }
    MOUNTS.lock().insert(
        path.clone(),
        fs_id
    );
    Ok(())
}

pub fn umount(path: &str) -> VfsResult<()> {
    MOUNTS.lock().remove(path);
    Ok(())
}

pub fn rebuild_path(path: &str) -> String {
    String::from("/")
        + &path
            .split("/")
            .fold(Vec::new(), |mut vec, x| {
                match x {
                    "" | "." => {}
                    ".." => {
                        if vec.len() > 0 {
                            vec.pop();
                        }
                    }
                    _ => vec.push(x),
                }
                vec
            })
            .join("/")
}

pub fn split_parent(path: &str) -> (String, String) {
    let path = rebuild_path(path);
    let rindex = path.rfind("/");
    if let Some(rindex) = rindex {
        (
            String::from(&path[..rindex]),
            String::from(&path[rindex + 1..]),
        )
    } else {
        (String::from("."), path)
    }
}

#[no_mangle]
pub fn open(path: &str) -> VfsResult<Arc<dyn INodeInterface>> {
    let path = rebuild_path(path);

    let mps = MOUNTS.lock().clone();
    for (mount_point, fs_id) in mps.iter().rev() {
        if path.starts_with(mount_point)
            && (mount_point == "/"
                || path.len() == mount_point.len()
                || path.chars().nth(mount_point.len()) == Some('/'))
        {
            let folder = FILESYSTEMS[*fs_id].root_dir();
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

#[no_mangle]
pub fn open_mount(path: &str) -> Option<Arc<dyn INodeInterface>> {
    debug!("open mount: {}", path);
    let mps = MOUNTS.lock().clone();
    for (mount_point, fs_id) in mps.iter().rev() {
        if mount_point == path {
            return Some(FILESYSTEMS[*fs_id].root_dir());
        }
    }
    None
}
