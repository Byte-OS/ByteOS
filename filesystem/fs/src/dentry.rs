use alloc::{sync::Arc, vec::Vec};
use sync::Mutex;
use vfscore::{FileSystem, INodeInterface, VfsResult};

use crate::pathbuf::PathBuf;

pub static MOUNTED_FS: Mutex<Vec<(PathBuf, DEntryNode)>> = Mutex::new(Vec::new());

#[derive(Clone)]
pub struct DEntryNode {
    pub fs: Arc<dyn FileSystem>,
    node: Arc<dyn INodeInterface>,
}

impl DEntryNode {
    #[inline]
    pub fn node(&self) -> Arc<dyn INodeInterface> {
        self.node.clone()
    }
}

/// 获取挂载的文件系统和挂载后的路径
///
/// # Arguments
///
/// - `path`  需要搜索的路径
///
/// # Returns
///
/// - [DEntryNode] `path` 对应挂载的文件系统
/// - [String]     `path` 减去挂载路径后的路径
///
pub fn get_mounted(path: &PathBuf) -> (DEntryNode, PathBuf) {
    let mounted = MOUNTED_FS.lock();
    let finded = mounted
        .iter()
        .rev()
        .find(|x| path.starts_with(&x.0))
        .unwrap();
    (finded.1.clone(), path.trim_start(&finded.0))
}

/// 挂载文件系统
///
/// # Arguments
///
/// - `fs`   需要挂载的文件系统
/// - `path` 文件系统挂载的路径
pub fn mount_fs(fs: Arc<dyn FileSystem>, path: &str) {
    let path = PathBuf::from(path);
    info!("SYSTEM FS mount {} @ {}", fs.name(), path);
    let node = fs.root_dir();
    MOUNTED_FS.lock().push((path, DEntryNode { fs, node }));
}

/// 取消挂载文件系统
///
/// # Arguments
///
/// - `path` 需要取消挂载的路径
pub fn umount(path: PathBuf) -> VfsResult<()> {
    MOUNTED_FS.lock().retain(|x| x.0 != path);
    Ok(())
}
