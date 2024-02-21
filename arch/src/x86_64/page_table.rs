use alloc::sync::Arc;
use x86::bits64::paging::{
    pd_index, pdpt_index, pml4_index, pt_index, PDEntry, PDFlags, PDPTEntry, PDPTFlags, PML4Entry,
    PML4Flags, PTEntry, PTFlags, PAGE_SIZE_ENTRIES,
};
use x86_64::instructions::tlb::flush_all;

use crate::{ArchInterface, MappingFlags, PhysAddr, PhysPage, VirtAddr, VirtPage, PAGE_SIZE, VIRT_ADDR_START};

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
        extern "C" {
            fn kernel_mapping_pdpt();
        }
        let pml4 = self.0.slice_mut_with_len::<PML4Entry>(PAGE_SIZE_ENTRIES);
        pml4[0x1ff] = PML4Entry((kernel_mapping_pdpt as u64 - VIRT_ADDR_START as u64) | 0x3);
    }

    #[inline]
    pub fn change(&self) {
        unsafe {
            core::arch::asm!(
                "
                    mov     cr3, {}
                ", 
                in(reg) self.0.0
            );
            flush_all();
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
                PML4Flags::P | PML4Flags::RW,
            );
        }

        let pdpt = PhysAddr::new(pml4[pml4_index].address().into())
            .slice_mut_with_len::<PDPTEntry>(PAGE_SIZE_ENTRIES);
        let pdpt_index = pdpt_index(vaddr);
        if !pdpt[pdpt_index].is_present() {
            pdpt[pdpt_index] = PDPTEntry::new(
                ArchInterface::frame_alloc_persist().to_addr().into(),
                PDPTFlags::P | PDPTFlags::RW,
            );
        }

        let pd = PhysAddr::new(pdpt[pdpt_index].address().into())
            .slice_mut_with_len::<PDEntry>(PAGE_SIZE_ENTRIES);
        let pd_index = pd_index(vaddr);
        if !pd[pd_index].is_present() {
            pd[pd_index] = PDEntry::new(
                ArchInterface::frame_alloc_persist().to_addr().into(),
                PDFlags::P | PDFlags::RW,
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
    }

    #[inline]
    pub fn unmap(&self, vpn: VirtPage) {
        *self.get_entry(vpn) = PTEntry(0);
    }

    #[inline]
    pub fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr {
        PhysAddr::new(self.get_entry(vaddr.into()).address().as_usize() + vaddr.0 % PAGE_SIZE)
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        todo!("PageTable Drop")
        // for root_pte in get_pte_list(self.0)[..0x100].iter().filter(|x| x.is_leaf()) {
        //     get_pte_list(root_pte.to_ppn().into())
        //         .iter()
        //         .filter(|x| x.is_leaf())
        //         .for_each(|x| ArchInterface::frame_unalloc(x.to_ppn()));
        //     ArchInterface::frame_unalloc(root_pte.to_ppn());
        // }
        // ArchInterface::frame_unalloc(self.0.into());
    }
}
