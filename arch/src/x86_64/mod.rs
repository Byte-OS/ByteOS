mod addr;
mod consts;
mod context;
mod entry;
mod interrupt;
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
    // #[cfg(platform = "x86_64-pc-oslab")]
    // {
    //     axlog::ax_println!("System will reboot, press any key to continue ...");
    //     while super::console::getchar().is_none() {}
    //     axlog::ax_println!("Rebooting ...");
    //     unsafe { PortWriteOnly::new(0x64).write(0xfeu8) };
    // }

    // #[cfg(platform = "x86_64-qemu-q35")]
    unsafe {
        PortWriteOnly::new(0x604).write(0x2000u16)
    };

    // crate::arch::halt();
    // warn!("It should shutdown!");
    // loop {
    //     crate::arch::halt();
    // }
    loop {

    }
}