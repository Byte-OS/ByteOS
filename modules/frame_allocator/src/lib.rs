#![no_std]

#[macro_use]
extern crate alloc;

use core::mem::size_of;

use alloc::vec::Vec;
use arch::{PAGE_SIZE, VIRT_ADDR_START, PhysPage};
use bit_field::{BitArray, BitField};
use log::info;
use devices::memory::get_memorys;
use sync::Mutex;

pub const fn floor(a: usize, b: usize) -> usize {
    return (a + b - 1) / b;
}

pub struct FrameRegionMap {
    bits: Vec<usize>,
    ppn: PhysPage
}

impl FrameRegionMap {
    pub fn new(start_addr: usize, end_addr: usize) -> Self {
        let mut bits = vec![0usize; floor((end_addr - start_addr) / PAGE_SIZE, 64)];
        
        // set non-exists memory bit as 1
        for i in (end_addr - start_addr) / PAGE_SIZE..bits.len() * 64 {
            bits.set_bit(i, true);
        }

        Self {
            bits,
            ppn: PhysPage::new(start_addr)
        }
    }

    pub fn get_free_page_count(&self) -> usize {
        self.bits.iter().fold(0, |mut sum, x| {
            if *x == 0 {
                sum + 64
            } else {
                for i in 0..64 {
                    sum += match (*x).get_bit(i) {
                        true => 0,
                        false => 1
                    };
                }
                sum
            }
        })
    }
}

pub struct FrameAllocator(Vec<FrameRegionMap>);

impl FrameAllocator {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add_memory_region(&mut self, start: usize, end: usize) {
        self.0.push(FrameRegionMap::new(start, end));
    }

    pub fn get_free_page_count(&self) -> usize {
        self.0.iter().fold(0, |sum, x| sum + x.get_free_page_count())
    }
}

pub static FRAME_ALLOCATOR: Mutex<FrameAllocator> = 
    Mutex::new(FrameAllocator::new());

pub fn init() {
    extern "C" {
        fn end();
    }
    let phys_end = floor(end as usize - VIRT_ADDR_START, PAGE_SIZE) * PAGE_SIZE;

    let mrs = get_memorys();

    mrs.iter().for_each(|mr| {
        if phys_end > mr.start && phys_end < mr.end {
            FRAME_ALLOCATOR.lock().add_memory_region(phys_end, mr.end);
        }
    });

    assert!(FRAME_ALLOCATOR.lock().0.len() > 0, "can't find frame to alloc");

    info!("free page count: {}", FRAME_ALLOCATOR.lock().get_free_page_count());
}
