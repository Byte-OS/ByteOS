#![no_std]

use buddy_system_allocator::LockedHeap;

// 堆大小
const HEAP_SIZE: usize = 0x0040_0000;

// 堆空间
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

// 堆内存分配器
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<64> = LockedHeap::empty();

// 初始化堆内存分配器
pub fn init() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP.as_ptr() as usize, HEAP_SIZE);
    }
}
