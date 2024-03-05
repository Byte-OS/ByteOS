use core::arch::asm;

use aarch64_cpu::asm::barrier;
use aarch64_cpu::registers::{Writeable, TTBR0_EL1};
use alloc::sync::Arc;
use bitflags::bitflags;

use crate::{
    ArchInterface, MappingFlags, PhysAddr, PhysPage, VirtAddr, VirtPage, PAGE_ITEM_COUNT, PAGE_SIZE,
};

use super::boot::flush_tlb;

#[derive(Copy, Clone, Debug)]
pub struct PTE(usize);

impl PTE {
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn from_ppn(ppn: usize, flags: PTEFlags) -> Self {
        PTE((ppn << 12) | flags.bits())
    }

    #[inline]
    pub const fn from_addr(addr: usize, flags: PTEFlags) -> Self {
        Self::from_ppn(addr >> 12, flags)
    }

    #[inline]
    pub const fn to_ppn(&self) -> PhysPage {
        PhysPage((self.0 & 0xffff_ffff_ffff_f000) >> 12)
    }

    #[inline]
    pub fn set(&mut self, ppn: usize, flags: PTEFlags) {
        self.0 = (ppn << 10) | flags.bits() as usize;
    }

    #[inline]
    pub const fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate(self.0)
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::VALID)
    }

    #[inline]
    pub fn is_block(&self) -> bool {
        self.flags().contains(PTEFlags::NON_BLOCK)
    }

    #[inline]
    pub fn get_next_ptr(&self) -> PhysAddr {
        PhysAddr(self.0 & 0xffff_ffff_f000)
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(value: MappingFlags) -> Self {
        let mut flags = PTEFlags::VALID | PTEFlags::NON_BLOCK | PTEFlags::AF | PTEFlags::NG;
        if !value.contains(MappingFlags::W) {
            flags |= PTEFlags::AP_RO;
        }

        if !value.contains(MappingFlags::X) {
            flags |= PTEFlags::UXN | PTEFlags::PXN;
        }

        if value.contains(MappingFlags::U) {
            flags |= PTEFlags::AP_EL0;
        }
        flags
    }
}

bitflags::bitflags! {
    /// Possible flags for a page table entry.
    pub struct PTEFlags: usize {
        // Attribute fields in stage 1 VMSAv8-64 Block and Page descriptors:
        /// Whether the descriptor is valid.
        const VALID =       1 << 0;
        /// The descriptor gives the address of the next level of translation table or 4KB page.
        /// (not a 2M, 1G block)
        const NON_BLOCK =   1 << 1;
        /// Memory attributes index field.
        const ATTR_INDX =   0b111 << 2;
        const NORMAL_NONCACHE = 0b010 << 2;
        /// Non-secure bit. For memory accesses from Secure state, specifies whether the output
        /// address is in Secure or Non-secure memory.
        const NS =          1 << 5;
        /// Access permission: accessable at EL0.
        const AP_EL0 =      1 << 6;
        /// Access permission: read-only.
        const AP_RO =       1 << 7;
        /// Shareability: Inner Shareable (otherwise Outer Shareable).
        const INNER =       1 << 8;
        /// Shareability: Inner or Outer Shareable (otherwise Non-shareable).
        const SHAREABLE =   1 << 9;
        /// The Access flag.
        const AF =          1 << 10;
        /// The not global bit.
        const NG =          1 << 11;
        /// Indicates that 16 adjacent translation table entries point to contiguous memory regions.
        const CONTIGUOUS =  1 <<  52;
        /// The Privileged execute-never field.
        const PXN =         1 <<  53;
        /// The Execute-never or Unprivileged execute-never field.
        const UXN =         1 <<  54;

        // Next-level attributes in stage 1 VMSAv8-64 Table descriptors:

        /// PXN limit for subsequent levels of lookup.
        const PXN_TABLE =           1 << 59;
        /// XN limit for subsequent levels of lookup.
        const XN_TABLE =            1 << 60;
        /// Access permissions limit for subsequent levels of lookup: access at EL0 not permitted.
        const AP_NO_EL0_TABLE =     1 << 61;
        /// Access permissions limit for subsequent levels of lookup: write access not permitted.
        const AP_NO_WRITE_TABLE =   1 << 62;
        /// For memory accesses from Secure state, specifies the Security state for subsequent
        /// levels of lookup.
        const NS_TABLE =            1 << 63;
    }
}

#[inline]
pub fn get_pte_list(paddr: PhysAddr) -> &'static mut [PTE] {
    unsafe { core::slice::from_raw_parts_mut(paddr.get_mut_ptr::<PTE>(), PAGE_ITEM_COUNT) }
}

#[derive(Debug)]
pub struct PageTable(pub(crate) PhysAddr);

impl PageTable {
    pub fn alloc() -> Arc<Self> {
        let addr = ArchInterface::frame_alloc_persist().into();
        let page_table = Self(addr);
        page_table.restore();
        Arc::new(page_table)
    }

    #[inline]
    pub fn restore(&self) {
        warn!("doing nothing")
    }

    #[inline]
    pub fn change(&self) {
        debug!("change ttbr0 to :{:#x}", self.0.addr());
        TTBR0_EL1.set((self.0.addr() & 0xFFFF_FFFF_F000) as _);
        unsafe { asm!("dsb ish;tlbi vmalle1is;") }
    }

    #[inline]
    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: MappingFlags, _level: usize) {
        let l1_list = self.0.slice_mut_with_len::<PTE>(512);
        let l1_index = (vpn.0 >> (9 * 2)) & 0x1ff;

        // l2 pte
        let l2_pte = &mut l1_list[l1_index];
        if !l2_pte.is_valid() {
            *l2_pte = PTE(ArchInterface::frame_alloc_persist().to_addr() | 0b11);
        }
        let l2_list = l2_pte.get_next_ptr().slice_mut_with_len::<PTE>(512);
        let l2_index = (vpn.0 >> 9) & 0x1ff;

        // l3 pte
        let l3_pte = &mut l2_list[l2_index];
        if !l3_pte.is_valid() {
            *l3_pte = PTE(ArchInterface::frame_alloc_persist().to_addr() | 0b11);
        }
        let l3_list = l3_pte.get_next_ptr().slice_mut_with_len::<PTE>(512);
        let l3_index = vpn.0 & 0x1ff;
        l3_list[l3_index] = PTE::from_ppn(ppn.0, flags.into());
    }

    #[inline]
    pub fn unmap(&self, vpn: VirtPage) {
        todo!("unmap pages");
        // TODO: Add huge page support.
        let mut pte_list = get_pte_list(self.0);
        for i in (1..3).rev() {
            let value = (vpn.0 >> 9 * i) & 0x1ff;
            let pte = &mut pte_list[value];
            if !pte.is_valid() {
                return;
            }
            pte_list = get_pte_list(pte.to_ppn().into());
        }

        pte_list[vpn.0 & 0x1ff] = PTE::new();
    }

    #[inline]
    pub fn virt_to_phys(&self, vaddr: VirtAddr) -> Option<PhysAddr> {
        let mut paddr = self.0;
        for i in (0..3).rev() {
            let value = (vaddr.0 >> (12 + 9 * i)) & 0x1ff;
            let pte = &get_pte_list(paddr)[value];
            // 如果当前页是大页 返回相关的位置
            // vaddr.0 % (1 << (12 + 9 * i)) 是大页内偏移
            if !pte.is_valid() {
                return None;
            }
            paddr = pte.to_ppn().into()
        }
        Some(PhysAddr(paddr.0 | vaddr.0 % PAGE_SIZE))
    }
}

// impl Drop for PageTable {
//     fn drop(&mut self) {
//         for root_pte in get_pte_list(self.0)[..0x100].iter().filter(|x| x.is_leaf()) {
//             get_pte_list(root_pte.to_ppn().into())
//                 .iter()
//                 .filter(|x| x.is_leaf())
//                 .for_each(|x| ArchInterface::frame_unalloc(x.to_ppn()));
//             ArchInterface::frame_unalloc(root_pte.to_ppn());
//         }
//         ArchInterface::frame_unalloc(self.0.into());
//     }
// }
