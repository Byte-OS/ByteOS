use fdt::node::FdtNode;

use crate::{Context, PhysPage, TrapType};

#[crate_interface::def_interface]
pub trait ArchInterface {
    /// kernel interrupt
    fn kernel_interrupt(ctx: &mut Context, trap_type: TrapType);
    /// init log
    fn init_logging();
    /// add a memory region
    fn add_memory_region(start: usize, end: usize);
    /// kernel main function, entry point.
    fn main(hartid: usize);
    /// Alloc a persistent memory page.
    fn frame_alloc_persist() -> PhysPage;
    /// Unalloc a persistent memory page
    fn frame_unalloc(ppn: PhysPage);
    /// Preprare drivers.
    fn prepare_drivers();
    /// Try to add device through FdtNode
    fn try_to_add_device(fdtNode: &FdtNode);
}
