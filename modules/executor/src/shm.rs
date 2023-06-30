use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use frame_allocator::FrameTracker;
use sync::Mutex;

pub struct SharedMemory {
    pub trackers: Vec<Arc<FrameTracker>>,
    pub deleted: Mutex<bool>,
}

impl SharedMemory {
    pub const fn new(trackers: Vec<Arc<FrameTracker>>) -> Self {
        Self {
            trackers,
            deleted: Mutex::new(false),
        }
    }
}

#[derive(Clone)]
pub struct MapedSharedMemory {
    pub key: usize,
    pub mem: Arc<SharedMemory>,
    pub start: usize,
    pub size: usize,
}

impl Drop for MapedSharedMemory {
    fn drop(&mut self) {
        // self.mem.trackers.remove(self.key);
        if Arc::strong_count(&self.mem) == 1 && *self.mem.deleted.lock() == true {
            SHARED_MEMORY.lock().remove(&self.key);
        }
    }
}

pub static SHARED_MEMORY: Mutex<BTreeMap<usize, Arc<SharedMemory>>> = Mutex::new(BTreeMap::new());
