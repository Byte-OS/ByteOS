use fdt::node::FdtNode;

use crate::{PhysPage, TrapFrame, TrapType};

/// ArchInterface
///
/// This trait indicates the interface was should be implemented
/// from the kernel layer.
///
/// You need to implement the interface manually.
///
/// eg: in kernel/src/main.rs
///
/// #[crate_interface::impl_interface]
/// impl ArchInterface for ArchInterfaceImpl {
///     /// Init allocator
///     fn init_allocator() {}
///     /// kernel interrupt
///     fn kernel_interrupt(ctx: &mut TrapFrame, trap_type: TrapType) {}
///     /// init log
///     fn init_logging() {}
///     /// add a memory region
///     fn add_memory_region(start: usize, end: usize) {}
///     /// kernel main function, entry point.
///     fn main(hartid: usize) {}
///     /// Alloc a persistent memory page.
///     fn frame_alloc_persist() -> PhysPage {}
///     /// Unalloc a persistent memory page
///     fn frame_unalloc(ppn: PhysPage) {}
///     /// Preprare drivers.
///     fn prepare_drivers() {}
///     /// Try to add device through FdtNode
///     fn try_to_add_device(_fdt_node: &FdtNode) {}
/// }

#[crate_interface::def_interface]
pub trait ArchInterface {
    /// Init allocator
    fn init_allocator();
    /// kernel interrupt
    fn kernel_interrupt(ctx: &mut TrapFrame, trap_type: TrapType);
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
