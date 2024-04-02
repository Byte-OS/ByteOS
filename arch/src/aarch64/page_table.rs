use aarch64_cpu::registers::{Writeable, TTBR0_EL1};

use crate::{
    pagetable::{pn_index, pn_offest, MappingFlags},
    ArchInterface, PhysAddr, PhysPage, VirtAddr, VirtPage, PAGE_ITEM_COUNT, PAGE_SIZE,
};

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

    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.flags().contains(PTEFlags::VALID | PTEFlags::NON_BLOCK)
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(value: MappingFlags) -> Self {
        let mut flags = PTEFlags::VALID | PTEFlags::NON_BLOCK | PTEFlags::AF;
        if !value.contains(MappingFlags::W) {
            flags |= PTEFlags::AP_RO;
        }

        if !value.contains(MappingFlags::X) {
            flags |= PTEFlags::UXN | PTEFlags::PXN;
        }

        if value.contains(MappingFlags::U) {
            flags |= PTEFlags::AP_EL0;
        }
        if !value.contains(MappingFlags::G) {
            flags |= PTEFlags::NG
        }
        flags
    }
}

impl Into<MappingFlags> for PTEFlags {
    fn into(self) -> MappingFlags {
        if self.is_empty() {
            return MappingFlags::empty();
        };
        let mut flags = MappingFlags::R;

        if !self.contains(PTEFlags::AP_RO) {
            flags |= MappingFlags::W;
        }
        if !self.contains(PTEFlags::UXN) || !self.contains(PTEFlags::PXN) {
            flags |= MappingFlags::X;
        }
        if self.contains(PTEFlags::AP_EL0) {
            flags |= MappingFlags::U;
        }
        if self.contains(PTEFlags::AF) {
            flags |= MappingFlags::A;
        }
        if !self.contains(PTEFlags::NG) {
            flags |= MappingFlags::G;
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

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PageTable(pub(crate) PhysAddr);

impl PageTable {
    #[inline]
    pub fn restore(&self) {
        let drop_l3 = |l3: PhysAddr| {
            l3.slice_mut_with_len::<PTE>(0x200)
                .iter_mut()
                .for_each(|x| *x = PTE(0));
        };
        let drop_l2 = |l2: PhysAddr| {
            l2.slice_mut_with_len::<PTE>(0x200).iter().for_each(|x| {
                if x.0 & 0b11 == 0b11 {
                    drop_l3(x.to_ppn().into())
                }
            })
        };
        self.0
            .slice_mut_with_len::<PTE>(0x200)
            .iter()
            .for_each(|x| {
                if x.0 & 0b11 == 0b11 {
                    drop_l2(x.to_ppn().into())
                }
            });
        flush_tlb(None)
    }

    #[inline]
    pub fn change(&self) {
        debug!("change ttbr0 to :{:#x}", self.0.addr());
        TTBR0_EL1.set((self.0.addr() & 0xFFFF_FFFF_F000) as _);
        flush_tlb(None)
    }

    pub fn get_mut_entry(&self, vpn: VirtPage) -> &mut PTE {
        let l1_list = self.0.slice_mut_with_len::<PTE>(512);
        let l1_index = pn_index(vpn, 2);

        // l2 pte
        let l2_pte = &mut l1_list[l1_index];
        if !l2_pte.is_valid() {
            *l2_pte = PTE(ArchInterface::frame_alloc_persist().to_addr() | 0b11);
        }
        let l2_list = l2_pte.get_next_ptr().slice_mut_with_len::<PTE>(512);
        let l2_index = pn_index(vpn, 1);

        // l3 pte
        let l3_pte = &mut l2_list[l2_index];
        if !l3_pte.is_valid() {
            *l3_pte = PTE(ArchInterface::frame_alloc_persist().to_addr() | 0b11);
        }
        let l3_list = l3_pte.get_next_ptr().slice_mut_with_len::<PTE>(512);
        let l3_index = pn_index(vpn, 0);
        &mut l3_list[l3_index]
    }

    #[inline]
    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: MappingFlags, _level: usize) {
        *self.get_mut_entry(vpn) = PTE::from_ppn(ppn.0, flags.into());
        flush_tlb(Some(vpn.into()))
    }

    #[inline]
    pub fn unmap(&self, vpn: VirtPage) {
        *self.get_mut_entry(vpn) = PTE(0);
        flush_tlb(Some(vpn.into()));
    }

    #[inline]
    pub fn virt_to_phys(&self, vaddr: VirtAddr) -> Option<PhysAddr> {
        let mut paddr = self.0;
        for i in (0..3).rev() {
            let value = pn_index(vaddr.into(), i);
            let pte = &get_pte_list(paddr)[value];
            // 如果当前页是大页 返回相关的位置
            // vaddr.0 % (1 << (12 + 9 * i)) 是大页内偏移
            if !pte.is_valid() {
                return None;
            };
            paddr = pte.to_ppn().into()
        }
        Some(PhysAddr(paddr.0 | vaddr.0 % PAGE_SIZE))
    }

    #[inline]
    pub fn translate(&self, vaddr: VirtAddr) -> Option<(PhysAddr, MappingFlags)> {
        let l3_pte = &get_pte_list(self.0)[pn_index(vaddr.into(), 2)];
        if !l3_pte.flags().contains(PTEFlags::VALID) {
            return None;
        };
        if !l3_pte.flags().contains(PTEFlags::NON_BLOCK) {
            return Some((
                PhysAddr(l3_pte.to_ppn().to_addr() | pn_offest(vaddr, 2)),
                l3_pte.flags().into(),
            ));
        }

        let l2_pte = get_pte_list(l3_pte.to_ppn().into())[pn_index(vaddr.into(), 1)];
        if !l2_pte.flags().contains(PTEFlags::VALID) {
            return None;
        };
        if !l2_pte.flags().contains(PTEFlags::NON_BLOCK) {
            return Some((
                PhysAddr(l2_pte.to_ppn().to_addr() | pn_offest(vaddr, 1)),
                l2_pte.flags().into(),
            ));
        }

        let l1_pte = get_pte_list(l2_pte.to_ppn().into())[pn_index(vaddr.into(), 0)];
        if !l1_pte.flags().contains(PTEFlags::VALID) {
            return None;
        };
        Some((
            PhysAddr(l1_pte.to_ppn().to_addr() | pn_offest(vaddr, 0)),
            l1_pte.flags().into(),
        ))
    }
}

impl PageTable {
    pub fn release(&self) {
        for root_pte in get_pte_list(self.0).iter().filter(|x| x.is_leaf()) {
            get_pte_list(root_pte.to_ppn().into())
                .iter()
                .filter(|x| x.is_leaf())
                .for_each(|x| ArchInterface::frame_unalloc(x.to_ppn()));
            ArchInterface::frame_unalloc(root_pte.to_ppn());
        }
        ArchInterface::frame_unalloc(self.0.into());
    }
}

#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    unsafe {
        if let Some(vaddr) = vaddr {
            // TIPS: flush tlb, tlb addr: 0-47: ppn, otherwise tlb asid
            core::arch::asm!("tlbi vaale1is, {}; dsb sy; isb", in(reg) ((vaddr.0 >> 12) & 0xFFFF_FFFF_FFFF))
        } else {
            // flush the entire TLB
            core::arch::asm!("tlbi vmalle1; dsb sy; isb")
        }
    }
}
