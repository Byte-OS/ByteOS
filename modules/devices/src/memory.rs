use core::sync::atomic::Ordering;

use fdt::Fdt;
use kheader::mm::{set_memory, MemoryRegion};

use crate::DEVICE_TREE_ADDR;

pub fn init() {
    let mut mrs = vec![];
    let fdt =
        unsafe { Fdt::from_ptr(DEVICE_TREE_ADDR.load(Ordering::Relaxed) as *const u8).unwrap() };

    fdt.memory().regions().for_each(|mr| {
        mrs.push(MemoryRegion {
            start: mr.starting_address as usize,
            end: mr.starting_address as usize + mr.size.unwrap_or(0),
        })
    });

    set_memory(mrs);
}
