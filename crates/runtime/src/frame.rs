use core::ops::Deref;

use alloc::vec::Vec;
use bit_field::{BitArray, BitField};
use log::info;
use polyhal::{consts::VIRT_ADDR_START, pa, pagetable::PAGE_SIZE, PhysAddr};
use sync::Mutex;

pub const fn alignup(a: usize, b: usize) -> usize {
    a.div_ceil(b) * b
}

pub const fn aligndown(a: usize, b: usize) -> usize {
    a / b * b
}

#[derive(Debug)]
/// 页帧
///
/// 用这个代表一个已经被分配的页表，并且利用 Drop 机制保证页表能够顺利被回收
pub struct FrameTracker(pub PhysAddr);

impl FrameTracker {
    pub const fn new(paddr: PhysAddr) -> Self {
        Self(paddr)
    }

    #[inline]
    pub fn clear(&self) {
        self.0.clear_len(PAGE_SIZE);
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        self.clear();
        FRAME_ALLOCATOR.lock().dealloc(self.0);
    }
}

impl Deref for FrameTracker {
    type Target = PhysAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// 页帧分布图
///
/// 利用页帧分布图保存页帧分配器中的空闲内存，并且利用 bitArray 记录页帧使用情况
pub struct FrameRegionMap {
    bits: Vec<usize>,
    paddr: PhysAddr,
    paddr_end: PhysAddr,
}

impl FrameRegionMap {
    /// 创建页帧分布图
    ///
    /// start_addr: usize 空闲页帧起始地址
    /// end_addr: usize 空闲页帧结束地址
    #[inline]
    pub fn new(start_addr: usize, end_addr: usize) -> Self {
        let mut bits = vec![0usize; ((end_addr - start_addr) / PAGE_SIZE).div_ceil(64)];

        // set non-exists memory bit as 1
        for i in (end_addr - start_addr) / PAGE_SIZE..bits.len() * 64 {
            bits.set_bit(i, true);
        }

        Self {
            bits,
            paddr: pa!(start_addr),
            paddr_end: pa!(end_addr),
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
    fn alloc_in_pos(&mut self, index: usize) -> Option<PhysAddr> {
        for bit_index in 0..64 {
            if !self.bits[index].get_bit(bit_index) {
                self.bits[index].set_bit(bit_index, true);
                return Some(pa!(self.paddr.raw() + (index * 64 + bit_index) * PAGE_SIZE));
            }
        }
        None
    }

    /// 申请一个空闲页
    #[inline]
    pub fn alloc(&mut self) -> Option<PhysAddr> {
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
        let start_ppn = self.paddr.raw() / PAGE_SIZE;
        let end_ppn = self.paddr_end.raw() / PAGE_SIZE;
        if pages > end_ppn - start_ppn {
            return None;
        }
        for mut i in 0..(end_ppn - start_ppn - pages + 1) {
            let mut j = i;
            loop {
                if j - i >= pages {
                    let mut ans = Vec::new();
                    (i..j).into_iter().for_each(|x| {
                        self.bits.set_bit(x, true);
                        ans.push(FrameTracker::new(pa!((start_ppn + x) * PAGE_SIZE)));
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
    pub fn dealloc(&mut self, paddr: PhysAddr) {
        let ppn = paddr.raw() / PAGE_SIZE;
        let start_ppn = self.paddr.raw() / PAGE_SIZE;
        self.bits.set_bit(ppn - start_ppn, false);
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
    pub fn alloc(&mut self) -> Option<PhysAddr> {
        self.0.iter_mut().find_map(|frm| frm.alloc())
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
    pub fn dealloc(&mut self, paddr: PhysAddr) {
        for frm in &mut self.0 {
            if paddr >= frm.paddr && paddr < frm.paddr_end {
                frm.dealloc(paddr);
                break;
            }
        }
    }
}

/// 一个总的页帧分配器
pub static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

pub fn add_frame_map(mut mm_start: usize, mut mm_end: usize) {
    mm_start = alignup(mm_start, PAGE_SIZE);
    mm_end = aligndown(mm_end, PAGE_SIZE);
    info!("add frame memory region {:#x} - {:#x}", mm_start, mm_end);

    FRAME_ALLOCATOR
        .lock()
        .add_memory_region(mm_start & !VIRT_ADDR_START, mm_end & !VIRT_ADDR_START);
}

/// 页帧分配器初始化
pub fn init() {
    info!("initialize frame allocator");

    // 确保帧分配器一定能工作
    assert!(
        FRAME_ALLOCATOR.lock().0.len() > 0,
        "can't find frame to alloc"
    );
}

/// 申请一个持久化存在的页表，需要手动释放
pub unsafe fn frame_alloc_persist() -> Option<PhysAddr> {
    FRAME_ALLOCATOR.lock().alloc()
}

/// 手动释放一个页表
pub unsafe fn frame_unalloc(paddr: PhysAddr) {
    FRAME_ALLOCATOR.lock().dealloc(paddr)
}

/// 申请一个空闲页表
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR.lock().alloc().map(FrameTracker)
}

/// 申请多个空闲连续页表
pub fn frame_alloc_much(pages: usize) -> Option<Vec<FrameTracker>> {
    FRAME_ALLOCATOR.lock().alloc_much(pages)
}

/// 获取空闲页表数量
pub fn get_free_pages() -> usize {
    FRAME_ALLOCATOR.lock().get_free_page_count()
}
