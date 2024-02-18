use crate::{Context, TrapType};

#[crate_interface::def_interface]
pub trait ArchInterface {
    fn interrupt_table() -> fn(&mut Context, TrapType);
    // fn add_device
    fn init_logging();
}
