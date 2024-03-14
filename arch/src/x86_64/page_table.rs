use alloc::sync::Arc;
use x86::bits64::paging::{
    pd_index, pdpt_index, pml4_index, pt_index, PDEntry, PDFlags, PDPTEntry, PDPTFlags, PML4Entry,
    PML4Flags, PTEntry, PTFlags, PAGE_SIZE_ENTRIES,
};

use crate::{
    flush_tlb, ArchInterface, MappingFlags, PhysAddr, PhysPage, VirtAddr, VirtPage, PAGE_SIZE,
    VIRT_ADDR_START,
};

impl From<MappingFlags> for PTFlags {
    fn from(flags: MappingFlags) -> Self {
        let mut res = Self::P;
        if flags.contains(MappingFlags::W) {
            res |= Self::RW;
        }
        if flags.contains(MappingFlags::U) {
            res |= Self::US;
        }
        if flags.contains(MappingFlags::A) {
            res |= Self::A;
        }
        if flags.contains(MappingFlags::D) {
            res |= Self::D;
        }
        if flags.contains(MappingFlags::X) {
            res.remove(Self::XD);
        }
        res
    }
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
        let map_pd = |pd_entry: &PDEntry| {
            if !pd_entry.is_present() || pd_entry.is_page() {
                return;
            }
            PhysAddr::new(pd_entry.address().as_usize())
                .slice_mut_with_len::<PTEntry>(PAGE_SIZE_ENTRIES)
                .iter_mut()
                .for_each(|x| *x = PTEntry(0));
        };

        let map_pdpt = |pdpt_entry: &PDPTEntry| {
            if !pdpt_entry.is_present() || pdpt_entry.is_page() {
                return;
            }
            PhysAddr::new(pdpt_entry.address().as_usize())
                .slice_mut_with_len::<PDEntry>(PAGE_SIZE_ENTRIES)
                .iter()
                .for_each(map_pd);
        };

        let map_pml4 = |pml4_entry: &mut PML4Entry| {
            if !pml4_entry.is_present() {
                return;
            }
            PhysAddr::new(pml4_entry.address().as_usize())
                .slice_mut_with_len::<PDPTEntry>(PAGE_SIZE_ENTRIES)
                .iter()
                .for_each(map_pdpt);
        };

        self.0.slice_mut_with_len::<PML4Entry>(PAGE_SIZE_ENTRIES)[..0x100]
            .iter_mut()
            .for_each(map_pml4);

        extern "C" {
            fn kernel_mapping_pdpt();
        }
        let pml4 = self.0.slice_mut_with_len::<PML4Entry>(PAGE_SIZE_ENTRIES);
        pml4[0x1ff] = PML4Entry((kernel_mapping_pdpt as u64 - VIRT_ADDR_START as u64) | 0x3);
        // mfence();
        flush_tlb(None);
    }

    #[inline]
    pub fn change(&self) {
        unsafe {
            core::arch::asm!("mov     cr3, {}", in(reg) self.0.0);
        }
    }

    #[inline]
    pub fn get_entry(&self, vpn: VirtPage) -> &mut PTEntry {
        let vaddr = vpn.to_addr().into();

        let pml4 = self.0.slice_mut_with_len::<PML4Entry>(PAGE_SIZE_ENTRIES);
        let pml4_index = pml4_index(vaddr);
        if !pml4[pml4_index].is_present() {
            pml4[pml4_index] = PML4Entry::new(
                ArchInterface::frame_alloc_persist().to_addr().into(),
                PML4Flags::P | PML4Flags::RW | PML4Flags::US,
            );
        }

        let pdpt = PhysAddr::new(pml4[pml4_index].address().into())
            .slice_mut_with_len::<PDPTEntry>(PAGE_SIZE_ENTRIES);
        let pdpt_index = pdpt_index(vaddr);
        if !pdpt[pdpt_index].is_present() {
            pdpt[pdpt_index] = PDPTEntry::new(
                ArchInterface::frame_alloc_persist().to_addr().into(),
                PDPTFlags::P | PDPTFlags::RW | PDPTFlags::US,
            );
        }

        let pd = PhysAddr::new(pdpt[pdpt_index].address().into())
            .slice_mut_with_len::<PDEntry>(PAGE_SIZE_ENTRIES);
        let pd_index = pd_index(vaddr);
        if !pd[pd_index].is_present() {
            pd[pd_index] = PDEntry::new(
                ArchInterface::frame_alloc_persist().to_addr().into(),
                PDFlags::P | PDFlags::RW | PDFlags::US,
            );
        }

        let pte = PhysAddr::new(pd[pd_index].address().into())
            .slice_mut_with_len::<PTEntry>(PAGE_SIZE_ENTRIES);
        let pte_index = pt_index(vaddr);
        &mut pte[pte_index]
    }

    #[inline]
    pub fn map(&self, ppn: PhysPage, vpn: VirtPage, flags: MappingFlags, _level: usize) {
        *self.get_entry(vpn) = PTEntry::new(ppn.to_addr().into(), flags.into());
        flush_tlb(Some(vpn.into()))
    }

    #[inline]
    pub fn unmap(&self, vpn: VirtPage) {
        *self.get_entry(vpn) = PTEntry(0);
        flush_tlb(Some(vpn.into()))
    }

    #[inline]
    pub fn virt_to_phys(&self, vaddr: VirtAddr) -> Option<PhysAddr> {
        let pte = self.get_entry(vaddr.into());
        if !pte.is_present() {
            None
        } else {
            Some(PhysAddr::new(
                pte.address().as_usize() + vaddr.0 % PAGE_SIZE,
            ))
        }
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        self.restore();
        ArchInterface::frame_unalloc(self.0.into());
    }
}
