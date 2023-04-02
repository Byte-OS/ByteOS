use alloc::vec::Vec;
use sync::Mutex;

static MEMORY_REGIONS: Mutex<Vec<MemoryRegion>> = Mutex::new(Vec::new());

#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: usize,
    pub end: usize,
}

#[inline]
pub fn get_memorys() -> Vec<MemoryRegion> {
    MEMORY_REGIONS.lock().clone()
}

pub fn set_memory(mrs: Vec<MemoryRegion>) {
    *MEMORY_REGIONS.lock() = mrs;
}
