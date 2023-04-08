use alloc::{collections::BTreeMap, string::String, sync::Arc};
use sync::Mutex;
use vfscore::{INodeInterface, OpenFlags, VfsError, VfsResult, FileType};

use crate::{FILESYSTEMS, File};

pub static MOUNTS: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());

pub fn init() {
    for (i, fs) in FILESYSTEMS.iter().enumerate() {
        match fs.name() {
            "fat32" => mount(String::from("/"), i).expect("can't mount to /"),
            "ramfs" => mount(String::from("/ramfs"), i).expect("can't mount to /ramfs"),
            "devfs" => mount(String::from("/devfs"), i).expect("can't mount to /devfs"),
            "procfs" => mount(String::from("/procfs"), i).expect("can't mount to /procfs"),
            _ => unreachable!(),
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
    MOUNTS.lock().insert(path, fs_id);
    Ok(())
}

pub fn open(path: &str) -> VfsResult<Arc<dyn INodeInterface>> {
    for (mount_point, id) in MOUNTS.lock().iter().rev() {
        if path.starts_with(mount_point) {
            let folder = FILESYSTEMS[*id].root_dir();

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
