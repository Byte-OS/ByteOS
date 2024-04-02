use core::ops::Deref;

use crate::{ArchInterface, PageTable, VirtAddr, VirtPage};

bitflags::bitflags! {
    /// Mapping flags for page table.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MappingFlags: u64 {
        /// User Accessable Flag
        const U = 1 << 0;
        /// Readable Flag
        const R = 1 << 1;
        /// Writeable Flag
        const W = 1 << 2;
        /// Executeable Flag
        const X = 1 << 3;
        /// Accessed Flag
        const A = 1 << 4;
        /// Dirty Flag, indicating that the page was written
        const D = 1 << 5;
        /// Global Flag
        const G = 1 << 6;
        /// Device Flag, indicating that the page was used for device memory
        const Device = 1 << 7;
        /// Cache Flag, indicating that the page will be cached
        const Cache = 1 << 8;

        /// Read | Write | Executeable Flags
        const RWX = Self::R.bits() | Self::W.bits() | Self::X.bits();
        /// User | Read | Write Flags
        const URW = Self::U.bits() | Self::R.bits() | Self::W.bits();
        /// User | Read | Executeable Flags
        const URX = Self::U.bits() | Self::R.bits() | Self::X.bits();
        /// User | Read | Write | Executeable Flags
        const URWX = Self::URW.bits() | Self::X.bits();
    }
}

/// Page Table Wrapper
///
/// You can use this wrapper to packing PageTable.
/// If you release the PageTableWrapper,
/// the PageTable will release its page table entry.
#[derive(Debug)]
pub struct PageTableWrapper(pub PageTable);

impl Deref for PageTableWrapper {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Allocate a new PageTableWrapper with new page table root
///
/// This operation will restore the page table.
impl PageTableWrapper {
    #[inline]
    pub fn alloc() -> Self {
        let pt = PageTable(ArchInterface::frame_alloc_persist().into());
        pt.restore();
        Self(pt)
    }
}

/// Indicates the page size.
pub enum MapPageSize {
    /// 4k Page
    Page4k,
    /// 2M Page
    Page2m,
    /// 1G Page
    Page1G,
}

/// Page Table Release.
///
/// You must implement this trait to release page table.
/// Include the page table entry and root page.
impl Drop for PageTableWrapper {
    fn drop(&mut self) {
        self.0.release();
    }
}

/// Get n level page table index of the given virtual address
pub fn pn_index(vpn: VirtPage, n: usize) -> usize {
    (vpn.0 >> 9 * n) & 0x1ff
}

/// Get n level page table offset of the given virtual address
pub fn pn_offest(vaddr: VirtAddr, n: usize) -> usize {
    vaddr.0 % (1 << (12 + 9 * n))
}

#[cfg(test)]
/// TODO: use #![feature(custom_test_frameworks)] to
/// Test this crate.
mod tests {
    use core::mem::size_of;

    use crate::pagetable::PageTableWrapper;

    #[test]
    fn exploration() {
        assert_eq!(
            size_of::<PageTableWrapper>(),
            size_of::<usize>(),
            "ensure the size of the page table wrapper equas the size of the usize"
        );
    }
}
