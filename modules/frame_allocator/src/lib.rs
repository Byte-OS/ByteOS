#![no_std]

#[macro_use]
extern crate alloc;

use core::mem::size_of;

use alloc::vec::Vec;
use arch::{PhysPage, PAGE_SIZE, VIRT_ADDR_START};
use bit_field::{BitArray, BitField};
use kheader::mm::get_memorys;
use log::info;
use sync::Mutex;

pub const fn floor(a: usize, b: usize) -> usize {
    return (a + b - 1) / b;
}

pub const fn ceil_div(a: usize, b: usize) -> usize {
    return (a + b - 1) / b;
}

#[derive(Debug)]
/// 页帧
///
/// 用这个代表一个已经被分配的页表，并且利用 Drop 机制保证页表能够顺利被回收
pub struct FrameTracker(pub PhysPage);

impl FrameTracker {
    pub fn new(ppn: PhysPage) -> Self {
        Self(ppn)
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        self.0.drop_clear();
        FRAME_ALLOCATOR.lock().dealloc(self.0);
    }
}

/// 页帧分布图
///
/// 利用页帧分布图保存页帧分配器中的空闲内存，并且利用 bitArray 记录页帧使用情况
pub struct FrameRegionMap {
    bits: Vec<usize>,
    ppn: PhysPage,
    ppn_end: PhysPage,
}

impl FrameRegionMap {
    /// 创建页帧分布图
    ///
    /// start_addr: usize 空闲页帧起始地址
    /// end_addr: usize 空闲页帧结束地址
    #[inline]
    pub fn new(start_addr: usize, end_addr: usize) -> Self {
        let mut bits = vec![0usize; floor((end_addr - start_addr) / PAGE_SIZE, 64)];

        // set non-exists memory bit as 1
        for i in (end_addr - start_addr) / PAGE_SIZE..bits.len() * 64 {
            bits.set_bit(i, true);
        }

        Self {
            bits,
            ppn: PhysPage::from_addr(start_addr),
            ppn_end: PhysPage::from_addr(end_addr),
        }
    }

    /// 获取页帧分布图中没有使用的页帧数量
    #[inline]
    pub fn get_free_page_count(&self) -> usize {
        self.bits.iter().fold(0, |mut sum, x| {
            if *x == 0 {
                sum + 64
            } else {
                for i in 0..64 {
                    sum += match (*x).get_bit(i) {
                        true => 0,
                        false => 1,
                    };
                }
                sum
            }
        })
    }

    /// 在 `bitArray` 指定位置获取一个空闲的页
    ///
    /// index: usize 指定的位置 self.bits[index]
    #[inline]
    fn alloc_in_pos(&mut self, index: usize) -> Option<FrameTracker> {
        for bit_index in 0..64 {
            if !self.bits[index].get_bit(bit_index) {
                self.bits[index].set_bit(bit_index, true);
                return Some(FrameTracker::new(self.ppn + index * 64 + bit_index));
            }
        }
        None
    }

    /// 申请一个空闲页
    #[inline]
    pub fn alloc(&mut self) -> Option<FrameTracker> {
        for i in 0..self.bits.len() {
            if self.bits[i] != usize::MAX {
                return self.alloc_in_pos(i);
            }
        }
        None
    }

    /// 申请多个空闲页, 空闲页是连续的
    ///
    /// pages: usize 要申请的页表数量
    #[allow(unused_assignments)]
    pub fn alloc_much(&mut self, pages: usize) -> Option<Vec<FrameTracker>> {
        // TODO: alloc more than 64?;
        // 优化本函数
        for mut i in 0..(usize::from(self.ppn_end) - usize::from(self.ppn) - pages + 1) {
            let mut j = i;
            loop {
                if j - i >= pages {
                    let mut ans = Vec::new();
                    (i..j).into_iter().for_each(|x| {
                        self.bits.set_bit(x, true);
                        ans.push(FrameTracker::new(self.ppn + x));
                    });
                    return Some(ans);
                }

                if self.bits.get_bit(j) == true {
                    i = j + 1;
                    break;
                }

                j += 1;
            }
        }
        None
    }

    /// 释放一个已经使用的页
    ///
    /// ppn: PhysPage 要释放的页的地址
    #[inline]
    pub fn dealloc(&mut self, ppn: PhysPage) {
        self.bits
            .set_bit(usize::from(ppn) - usize::from(self.ppn), false);
    }
}

/// 一个总的页帧分配器
pub struct FrameAllocator(Vec<FrameRegionMap>);

impl FrameAllocator {
    /// 创建一个空闲的页帧分配器
    #[inline]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// 将一块内存放在页帧分配器上
    ///
    /// start: usize 内存的起始地址
    /// end: usize 内存的结束地址
    #[inline]
    pub fn add_memory_region(&mut self, start: usize, end: usize) {
        self.0.push(FrameRegionMap::new(start, end));
    }

    /// 获取页帧分配器中空闲页表的数量
    ///
    /// 也就是对所有的页帧分布图中的内存进行和运算
    #[inline]
    pub fn get_free_page_count(&self) -> usize {
        self.0
            .iter()
            .fold(0, |sum, x| sum + x.get_free_page_count())
    }

    /// 申请一个空闲页
    #[inline]
    pub fn alloc(&mut self) -> Option<FrameTracker> {
        for frm in &mut self.0 {
            let frame = frm.alloc();
            if frame.is_some() {
                return frame;
            }
        }
        None
    }

    /// 申请多个空闲页, 空闲页是连续的
    ///
    /// pages: usize 要申请的页表数量
    /// 在多个页表分布图里查找
    #[inline]
    pub fn alloc_much(&mut self, pages: usize) -> Option<Vec<FrameTracker>> {
        for frm in &mut self.0 {
            let frame = frm.alloc_much(pages);
            if frame.is_some() {
                return frame;
            }
        }
        None
    }

    /// 释放一个页
    #[inline]
    pub fn dealloc(&mut self, ppn: PhysPage) {
        for frm in &mut self.0 {
            if ppn >= frm.ppn && ppn < frm.ppn_end {
                frm.dealloc(ppn);
                break;
            }
        }
    }
}

/// 一个总的页帧分配器
pub static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

/// 页帧分配器初始化
pub fn init() {
    extern "C" {
        fn end();
    }
    info!("initialize frame allocator");
    let phys_end = floor(end as usize - VIRT_ADDR_START, PAGE_SIZE) * PAGE_SIZE;

    // 从设备树中获取内存分布
    let mrs = get_memorys();

    // 在帧分配器中添加内存
    mrs.iter().for_each(|mr| {
        if phys_end > mr.start && phys_end < mr.end {
            unsafe {
                core::slice::from_raw_parts_mut(
                    phys_end as *mut usize,
                    (mr.end - phys_end) / size_of::<usize>(),
                )
                .fill(0);
            };
            FRAME_ALLOCATOR.lock().add_memory_region(phys_end, mr.end);
        }
    });

    // 确保帧分配器一定能工作
    assert!(
        FRAME_ALLOCATOR.lock().0.len() > 0,
        "can't find frame to alloc"
    );
}

/// 申请一个空闲页表
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR.lock().alloc()
}

/// 申请多个空闲连续页表
pub fn frame_alloc_much(pages: usize) -> Option<Vec<FrameTracker>> {
    FRAME_ALLOCATOR.lock().alloc_much(pages)
}

/// 获取空闲页表数量
pub fn get_free_pages() -> usize {
    FRAME_ALLOCATOR.lock().get_free_page_count()
}
