//! Define dynamic memory allocation.

use alloc::alloc::handle_alloc_error;
use sync::Mutex;
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};
use customizable_buddy::{BuddyAllocator, LinkedListBuddy, UsizeBuddy};

/// 堆分配器。
///
/// 27 + 6 + 3 = 36 -> 64 GiB
struct LockedHeap(Mutex<BuddyAllocator<27, UsizeBuddy, LinkedListBuddy>>);

#[global_allocator]
static HEAP: LockedHeap = LockedHeap(Mutex::new(BuddyAllocator::new()));

/// 为启动准备的初始内存。
///
/// 经测试，不同硬件的需求：
///
/// | machine         | memory
/// | --------------- | -
/// | qemu,virt SMP 1 |  16 KiB
/// | qemu,virt SMP 4 |  32 KiB
/// | allwinner,nezha | 256 KiB
unsafe impl GlobalAlloc for LockedHeap {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if let Ok((ptr, _)) = self.0.lock().allocate_layout(layout) {
            ptr.as_ptr()
        } else {
            handle_alloc_error(layout)
        }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0
            .lock()
            .deallocate_layout(NonNull::new(ptr).unwrap(), layout)
    }
}

/// 初始化分配器，并将一个小的内存块注册到分配器中，用于启动需要的动态内存。
pub fn init() {
    unsafe {
        log::info!("MEMORY = {:#?}", crate::HEAP.as_ptr_range());
        let mut heap = HEAP.0.lock();
        let ptr = NonNull::new(crate::HEAP.as_mut_ptr()).unwrap();
        heap.init(core::mem::size_of::<usize>().trailing_zeros() as _, ptr);
        heap.transfer(ptr, crate::HEAP.len());
    }
}

// /// 将一些内存区域注册到分配器。
// pub fn insert_regions(regions: &[Range<PhysAddr>]) {
//     let mut heap = HEAP.0.lock();
//     let offset = phys_to_virt_offset();
//     regions
//         .iter()
//         .filter(|region| !region.is_empty())
//         .for_each(|region| unsafe {
//             heap.transfer(
//                 NonNull::new_unchecked((region.start + offset) as *mut u8),
//                 region.len(),
//             );
//         });
// }
