#![no_std]

extern crate alloc;

use buddy_system_allocator::LockedHeap;
use log::info;

include!(concat!(env!("OUT_DIR"), "/consts.rs"));

// 堆大小
// const HEAP_SIZE: usize = 0x0180_0000;
// pub const HEAP_SIZE: usize = 0x0180_0000;

// 堆空间
#[link_section = ".bss.heap"]
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// 堆内存分配器
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<30> = LockedHeap::empty();

/// 初始化堆内存分配器
pub fn init() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP.as_mut_ptr() as usize, HEAP_SIZE);

        info!(
            "kernel HEAP init: {:#x} - {:#x}  size: {:#x}",
            HEAP.as_ptr() as usize,
            HEAP.as_ptr() as usize + HEAP_SIZE,
            HEAP_SIZE
        );
    }
}
