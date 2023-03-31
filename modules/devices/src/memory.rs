use core::sync::atomic::Ordering;

use alloc::vec::Vec;
use fdt::Fdt;

use crate::DEVICE_TREE_ADDR;

pub struct MemoryRegion {
    pub start: usize,
    pub end: usize,
}

pub fn get_memorys() -> Vec<MemoryRegion> {
    let mut mrs = vec![];
    let fdt =
        unsafe { Fdt::from_ptr(DEVICE_TREE_ADDR.load(Ordering::Relaxed) as *const u8).unwrap() };

    fdt.memory().regions().for_each(|mr| {
        mrs.push(MemoryRegion {
            start: mr.starting_address as usize,
            end: mr.starting_address as usize + mr.size.unwrap_or(0),
        })
    });

    mrs
}
