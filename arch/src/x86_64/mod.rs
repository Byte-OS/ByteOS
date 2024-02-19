mod addr;
mod consts;
mod context;
mod entry;
mod interrupt;
mod idt;
mod trap;
mod pic;
mod page_table;
mod uart;
mod sigtrx;
mod multiboot;

pub use addr::*;
pub use consts::*;
pub use context::Context;
pub use interrupt::*;
pub use page_table::*;
pub use uart::*;
use x86_64::instructions::port::PortWriteOnly;

pub fn shutdown() -> ! {
    unsafe {
        PortWriteOnly::new(0x604).write(0x2000u16)
    };

    loop {

    }
}