use crate::{Context, PhysPage, TrapType};

#[crate_interface::def_interface]
pub trait ArchInterface {
    fn interrupt_table() -> fn(&mut Context, TrapType);
    // fn add_device
    fn init_logging();
    /// add a memory region
    fn add_memory_region(start: usize, end: usize);
    /// kernel main function, entry point.
    fn main(hartid: usize, device_tree: usize);
    /// Alloc a persistent memory page.
    fn frame_alloc_persist() -> Option<PhysPage>;
    /// Unalloc a persistent memory page
    fn frame_unalloc(ppn: PhysPage);
}
