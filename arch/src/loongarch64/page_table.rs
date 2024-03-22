use alloc::sync::Arc;
use loongarch64::register::pgdl;

use crate::{
    ArchInterface, MappingFlags, PhysAddr, PhysPage, VirtAddr, VirtPage, PAGE_ITEM_COUNT, PAGE_SIZE,
};

use super::sigtrx::get_trx_mapping;

#[derive(Copy, Clone, Debug)]
pub struct PTE(pub usize);

impl PTE {
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn from_addr(ppn: PhysAddr, flags: PTEFlags) -> Self {
        PTE(ppn.0 | flags.bits())
    }

    #[inline]
    pub const fn addr(&self) -> PhysAddr {
        PhysAddr(self.0 & 0xffff_ffff_ffff_f000)
    }

    #[inline]
    pub const fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate(self.0)
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub fn get_next_ptr(&self) -> PhysAddr {
        PhysAddr(self.0 & 0xffff_ffff_f000)
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(value: MappingFlags) -> Self {
        let mut flags = PTEFlags::V;
        if value.contains(MappingFlags::W) {
            flags |= PTEFlags::W | PTEFlags::D;
        }

        if !value.contains(MappingFlags::X) {
            flags |= PTEFlags::NX;
        }

        if value.contains(MappingFlags::U) {
            flags |= PTEFlags::PLV_USER;
        }
        flags
    }
}

bitflags::bitflags! {
    /// Possible flags for a page table entry.
    pub struct PTEFlags: usize {
        /// Page Valid
        const V = 1 << 0;
        /// Dirty, The page has been writed.
        const D = 1 << 1;

        const PLV_USER = 0b11 << 2;

        const MAT_NOCACHE = 0b01 << 4;

        /// Designates a global mapping OR Whether the page is huge page.
        const GH = 1 << 6;

        /// Page is existing.
        const P = 1 << 7;
        /// Page is writeable.
        const W = 1 << 8;
        /// Is a Global Page if using huge page(GH bit).
        const G = 1 << 10;
        /// Page is not readable.
        const NR = 1 << 11;
        /// Page is not executable.
        const NX = 1 << 12;
        /// Whether the privilege Level is restricted. When RPLV is 0, the PTE
        /// can be accessed by any program with privilege Level highter than PLV.
        const RPLV = 1 << 63;
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
        let clear_l3 = |l3_ptr: &PTE| {
            if !l3_ptr.is_valid() {
                return;
            }
            l3_ptr
                .get_next_ptr()
                .slice_mut_with_len::<PTE>(0x200)
                .fill_with(|| PTE(0));
        };
        self.0
            .slice_mut_with_len::<PTE>(0x199)
            .iter()
            .for_each(|l1_pte| {
                if !l1_pte.is_valid() {
                    return;
                }
                l1_pte
                    .get_next_ptr()
                    .slice_mut_with_len::<PTE>(0x200)
                    .iter()
                    .for_each(clear_l3);
            });
        self.0.slice_mut_with_len::<PTE>(0x200)[0x100] = PTE(get_trx_mapping());

        flush_tlb(None);
    }

    #[inline]
    pub fn change(&self) {
        pgdl::set_base(self.0.addr());
        flush_tlb(None);
    }

    #[inline]
    pub fn get_mut_entry(&self, vpn: VirtPage) -> &mut PTE {
        let l1_list = self.0.slice_mut_with_len::<PTE>(512);
        let l1_index = (vpn.0 >> (9 * 2)) & 0x1ff;

        // l2 pte
        let l2_pte = &mut l1_list[l1_index];
        if !l2_pte.is_valid() {
            *l2_pte = PTE(ArchInterface::frame_alloc_persist().to_addr());
        }
        let l2_list = l2_pte.get_next_ptr().slice_mut_with_len::<PTE>(512);
        let l2_index = (vpn.0 >> 9) & 0x1ff;

        // l3 pte
        let l3_pte = &mut l2_list[l2_index];
        if !l3_pte.is_valid() {
            *l3_pte = PTE(ArchInterface::frame_alloc_persist().to_addr());
        }
        let l3_list = l3_pte.get_next_ptr().slice_mut_with_len::<PTE>(512);
        let l3_index = vpn.0 & 0x1ff;
        &mut l3_list[l3_index]
    }

    #[inline]
    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: MappingFlags, _level: usize) {
        *self.get_mut_entry(vpn) = PTE::from_addr(ppn.into(), flags.into());
        flush_tlb(Some(vpn.into()))
    }

    #[inline]
    pub fn unmap(&self, vpn: VirtPage) {
        *self.get_mut_entry(vpn) = PTE(0);
        flush_tlb(Some(vpn.into()))
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
            paddr = pte.addr()
        }
        Some(PhysAddr(paddr.0 | vaddr.0 % PAGE_SIZE))
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        for root_pte in get_pte_list(self.0)[..0x199]
            .iter()
            .filter(|x| x.is_valid())
        {
            get_pte_list(root_pte.addr())
                .iter()
                .filter(|x| x.is_valid())
                .for_each(|x| ArchInterface::frame_unalloc(x.addr().into()));
            ArchInterface::frame_unalloc(root_pte.addr().into());
        }
        ArchInterface::frame_unalloc(self.0.into());
    }
}

#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    if let Some(vaddr) = vaddr {
        unsafe {
            core::arch::asm!("dbar 0; invtlb 0x05, $r0, {reg}", reg = in(reg) vaddr.0);
        }
    } else {
        unsafe {
            core::arch::asm!("dbar 0; invtlb 0x00, $r0, $r0");
        }
    }
}

pub fn switch_to_kernel_page_table() {
    // todo!("switch to kernel page table")
    pgdl::set_base(0);
    flush_tlb(None);
}
