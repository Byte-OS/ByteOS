use bitflags::bitflags;
use riscv::register::satp;

use crate::currrent_arch::entry::PAGE_TABLE;
use crate::pagetable::{pn_index, pn_offest};
use crate::{
    flush_tlb, pagetable::MappingFlags, sigtrx::get_trx_mapping, ArchInterface, PhysAddr, PhysPage,
    VirtAddr, VirtPage, PAGE_ITEM_COUNT, VIRT_ADDR_START,
};

pub fn map_kernel(vpn: VirtPage, flags: MappingFlags) {
    let ppn = ArchInterface::frame_alloc_persist();
    let page_table = PageTable(crate::PhysAddr(satp::read().ppn() << 12));
    page_table.map(ppn, vpn, flags, 3);
}

#[derive(Copy, Clone, Debug)]
pub struct PTE(usize);

impl PTE {
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn from_ppn(ppn: usize, flags: PTEFlags) -> Self {
        // let flags = flags.union(PTEFlags::D);
        let mut flags = flags;
        if flags.contains(PTEFlags::R) | flags.contains(PTEFlags::X) {
            flags = flags.union(PTEFlags::A)
        }
        if flags.contains(PTEFlags::W) {
            flags = flags.union(PTEFlags::D)
        }
        // TIPS: This is prepare for the extend bits of T-HEAD C906
        #[cfg(c906)]
        if flags.contains(PTEFlags::G) && ppn == 0x8_0000 {
            Self(
                ppn << 10
                    | flags
                        .union(PTEFlags::C)
                        .union(PTEFlags::B)
                        .union(PTEFlags::K)
                        .bits() as usize,
            )
        } else if flags.contains(PTEFlags::G) && ppn == 0 {
            Self(ppn << 10 | flags.union(PTEFlags::SE).union(PTEFlags::SO).bits() as usize)
        } else {
            Self(ppn << 10 | flags.union(PTEFlags::C).bits() as usize)
        }

        #[cfg(not(c906))]
        Self(ppn << 10 | flags.bits() as usize)
    }

    #[inline]
    pub const fn from_addr(addr: usize, flags: PTEFlags) -> Self {
        Self::from_ppn(addr >> 12, flags)
    }

    #[inline]
    pub const fn to_ppn(&self) -> PhysPage {
        PhysPage((self.0 >> 10) & ((1 << 29) - 1))
    }

    #[inline]
    pub fn set(&mut self, ppn: usize, flags: PTEFlags) {
        self.0 = (ppn << 10) | flags.bits() as usize;
    }

    #[inline]
    pub const fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate((self.0 & 0xff) as u64)
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::V) && self.0 > u8::MAX as usize
    }

    /// 判断是否是大页
    ///
    /// 大页判断条件 V 位为 1, R/W/X 位至少有一个不为 0
    /// PTE 页表范围 1G(0x4000_0000) 2M(0x20_0000) 4K(0x1000)
    #[inline]
    pub fn is_huge(&self) -> bool {
        return self.flags().contains(PTEFlags::V)
            && (self.flags().contains(PTEFlags::R)
                || self.flags().contains(PTEFlags::W)
                || self.flags().contains(PTEFlags::X));
    }

    #[inline]
    pub fn is_leaf(&self) -> bool {
        return self.flags().contains(PTEFlags::V)
            && !(self.flags().contains(PTEFlags::R)
                || self.flags().contains(PTEFlags::W)
                || self.flags().contains(PTEFlags::X));
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PTEFlags: u64 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;

        #[cfg(c906)]
        const SO = 1 << 63;
        #[cfg(c906)]
        const C = 1 << 62;
        #[cfg(c906)]
        const B = 1 << 61;
        #[cfg(c906)]
        const K = 1 << 60;
        #[cfg(c906)]
        const SE = 1 << 59;

        const VRWX  = Self::V.bits() | Self::R.bits() | Self::W.bits() | Self::X.bits();
        const ADUVRX = Self::A.bits() | Self::D.bits() | Self::U.bits() | Self::V.bits() | Self::R.bits() | Self::X.bits();
        const ADVRWX = Self::A.bits() | Self::D.bits() | Self::VRWX.bits();
        const ADGVRWX = Self::G.bits() | Self::ADVRWX.bits();
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(flags: MappingFlags) -> Self {
        if flags.is_empty() {
            Self::empty()
        } else {
            let mut res = Self::V;
            if flags.contains(MappingFlags::R) {
                res |= PTEFlags::R;
            }
            if flags.contains(MappingFlags::W) {
                res |= PTEFlags::W;
            }
            if flags.contains(MappingFlags::X) {
                res |= PTEFlags::X;
            }
            if flags.contains(MappingFlags::U) {
                res |= PTEFlags::U;
            }
            if flags.contains(MappingFlags::A) {
                res |= PTEFlags::A;
            }
            if flags.contains(MappingFlags::D) {
                res |= PTEFlags::D;
            }
            res
        }
    }
}

impl From<PTEFlags> for MappingFlags {
    fn from(value: PTEFlags) -> Self {
        let mut mapping_flags = MappingFlags::empty();
        if value.contains(PTEFlags::R) {
            mapping_flags |= MappingFlags::R;
        }
        if value.contains(PTEFlags::W) {
            mapping_flags |= MappingFlags::W;
        }
        if value.contains(PTEFlags::X) {
            mapping_flags |= MappingFlags::X;
        }
        if value.contains(PTEFlags::U) {
            mapping_flags |= MappingFlags::U;
        }
        if value.contains(PTEFlags::A) {
            mapping_flags |= MappingFlags::A;
        }
        if value.contains(PTEFlags::D) {
            mapping_flags |= MappingFlags::D;
        }

        mapping_flags
    }
}

#[inline]
pub fn get_pte_list(paddr: PhysAddr) -> &'static mut [PTE] {
    unsafe { core::slice::from_raw_parts_mut(paddr.get_mut_ptr::<PTE>(), PAGE_ITEM_COUNT) }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PageTable(pub(crate) PhysAddr);

impl PageTable {
    pub fn current() -> Self {
        Self(PhysAddr(satp::read().ppn() << 12))
    }

    pub fn token(&self) -> usize {
        (8 << 60) | (self.0 .0 >> 12)
    }

    #[inline]
    pub fn restore(&self) {
        let arr = get_pte_list(self.0);
        arr[0x100] = PTE::from_addr(0x0000_0000, PTEFlags::ADGVRWX);
        arr[0x101] = PTE::from_addr(0x4000_0000, PTEFlags::ADGVRWX);
        arr[0x102] = PTE::from_addr(0x8000_0000, PTEFlags::ADGVRWX);
        arr[0x104] = PTE::from_addr(get_trx_mapping(), PTEFlags::V);
        arr[0x106] = PTE::from_addr(0x8000_0000, PTEFlags::ADGVRWX);
        // arr[0..0x100].fill(PTE::from_addr(0, PTEFlags::empty()));
        arr[0..0x100].fill(PTE(0));
    }

    #[inline]
    pub fn change(&self) {
        // Write page table entry for
        satp::write((8 << 60) | (self.0 .0 >> 12));
        flush_tlb(None);
    }

    #[inline]
    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: MappingFlags, level: usize) {
        // TODO: Add huge page support.
        let mut pte_list = get_pte_list(self.0);
        for i in (1..level).rev() {
            let pte = &mut pte_list[pn_index(vpn, i)];
            if i == 0 {
                break;
            }
            if !pte.is_valid() {
                *pte = PTE::from_ppn(ArchInterface::frame_alloc_persist().0, PTEFlags::V);
            }

            // page_table = PageTable(pte.to_ppn().into());
            pte_list = get_pte_list(pte.to_ppn().into());
        }

        pte_list[pn_index(vpn, 0)] = PTE::from_ppn(ppn.0, flags.into());
        flush_tlb(Some(vpn.into()))
    }

    #[inline]
    pub fn unmap(&self, vpn: VirtPage) {
        // TODO: Add huge page support.
        let mut pte_list = get_pte_list(self.0);
        for i in (1..3).rev() {
            let pte = &mut pte_list[pn_index(vpn, i)];
            if !pte.is_valid() {
                return;
            }
            pte_list = get_pte_list(pte.to_ppn().into());
        }

        pte_list[pn_index(vpn, 0)] = PTE::new();
        flush_tlb(Some(vpn.into()))
    }

    #[inline]
    pub fn translate(&self, vaddr: VirtAddr) -> Option<(PhysAddr, MappingFlags)> {
        let l3_pte = &get_pte_list(self.0)[pn_index(vaddr.into(), 2)];
        if !l3_pte.flags().contains(PTEFlags::V) {
            return None;
        };
        if l3_pte.is_huge() {
            return Some((
                PhysAddr(l3_pte.to_ppn().to_addr() | pn_offest(vaddr, 2)),
                l3_pte.flags().into(),
            ));
        }

        let l2_pte = get_pte_list(l3_pte.to_ppn().into())[pn_index(vaddr.into(), 1)];
        if !l2_pte.flags().contains(PTEFlags::V) {
            return None;
        };
        if l2_pte.is_huge() {
            return Some((
                PhysAddr(l2_pte.to_ppn().to_addr() | pn_offest(vaddr, 1)),
                l2_pte.flags().into(),
            ));
        }

        let l1_pte = get_pte_list(l2_pte.to_ppn().into())[pn_index(vaddr.into(), 0)];
        if !l1_pte.flags().contains(PTEFlags::V) {
            return None;
        };
        Some((
            PhysAddr(l1_pte.to_ppn().to_addr() | pn_offest(vaddr, 0)),
            l1_pte.flags().into(),
        ))
    }
}

impl PageTable {
    pub(crate) fn release(&self) {
        unsafe {
            if self.0.addr() == (PAGE_TABLE.as_ptr() as usize & !VIRT_ADDR_START) {
                return;
            }
        }
        for root_pte in get_pte_list(self.0)[..0x100].iter().filter(|x| x.is_leaf()) {
            get_pte_list(root_pte.to_ppn().into())
                .iter()
                .filter(|x| x.is_leaf())
                .for_each(|x| ArchInterface::frame_unalloc(x.to_ppn()));
            ArchInterface::frame_unalloc(root_pte.to_ppn());
        }
        ArchInterface::frame_unalloc(self.0.into());
    }
}
