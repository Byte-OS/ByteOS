use crate::{Context, TrapType};

#[crate_interface::def_interface]
pub trait ArchInterface {
    fn interrupt_table() -> Option<fn(&mut Context, TrapType)>;
    // fn add_device
    fn init_logging();
}

pub fn prepare_init() {
    crate_interface::call_interface!(ArchInterface::init_logging);
    // Init allocator
    allocator::init();
}
