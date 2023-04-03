use bitflags::{bitflags, BitFlags};

use crate::{PhysAddr, PhysPage, VirtAddr, VirtPage, PAGE_ITEM_COUNT, PAGE_SIZE};

#[derive(Copy, Clone)]
pub struct PTE(usize);

impl PTE {
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn from_ppn(ppn: usize, flags: PTEFlags) -> Self {
        Self(ppn << 10 | flags.bits() as usize)
    }

    #[inline]
    pub const fn from_addr(addr: usize, flags: PTEFlags) -> Self {
        Self::from_ppn(addr >> 12, flags)
    }

    #[inline]
    pub fn to_ppn(&self) -> PhysPage {
        PhysPage(self.0 >> 10)
    }

    #[inline]
    pub fn set(&mut self, ppn: usize, flags: PTEFlags) {
        self.0 = (ppn << 10) | flags.bits() as usize;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;

        const VRWX  = Self::V.bits() | Self::R.bits() | Self::W.bits() | Self::X.bits();
        const UVRWX = Self::U.bits() | Self::VRWX.bits();
        const GVRWX = Self::G.bits() | Self::VRWX.bits();

        const NONE  = 0;
    }
}

pub struct PageTable(PhysAddr);

impl PageTable {
    pub fn new(addr: PhysAddr) -> Self {
        Self(addr)
    }

    pub fn from_ppn(ppn: PhysPage) -> Self {
        Self(PhysAddr(ppn.0 << 12))
    }

    /// TODO physpage or virtpage
    #[inline]
    pub fn get_pte_list(&self) -> &'static mut [PTE] {
        unsafe { core::slice::from_raw_parts_mut(self.0 .0 as *mut PTE, PAGE_ITEM_COUNT) }
    }

    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: PTEFlags, falloc: fn() -> PhysPage) {
        for i in (0..3).rev() {
            // let page_table = PageTable(acc);
            // let value = vpn.0 >> x * 9;
            // let pte = &page_table.get_pte_list()[value];
        }
    }

    #[inline]
    pub fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr {
        let offset = vaddr.0 % PAGE_SIZE;
        let paddr = (2..-1).fold(self.0, |acc, x| {
            let page_table = PageTable(acc);
            let value = vaddr.0 >> 9 * x >> 10 & 0x1ff;
            let pte = &page_table.get_pte_list()[value];
            pte.to_ppn().into()
        });
        PhysAddr(paddr.0 | offset)
    }
}
