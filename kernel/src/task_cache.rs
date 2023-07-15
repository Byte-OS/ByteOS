#![allow(dead_code)]

/// This mod is due to implemente the exec map,
/// just need to map this file to accelerate the execution.
use alloc::{string::String, sync::Arc, vec::Vec};
use arch::VirtPage;
use frame_allocator::FrameTracker;
use fs::mount::{open, rebuild_path};
use sync::Mutex;

use crate::syscall::consts::{from_vfs, LinuxError};

pub struct TaskFile {
    pub filename: String,
    pub pages: Vec<(VirtPage, Vec<Arc<FrameTracker>>)>,
}

pub static MAP_CACHE_TABLE: Mutex<Vec<TaskFile>> = Mutex::new(Vec::new());

pub fn cache_exec_file(path: &str) -> Result<(), LinuxError> {
    let path = rebuild_path(path);

    let _file = open(&path).map_err(from_vfs)?;

    Ok(())
}
