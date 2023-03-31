#![no_main]
#![no_std]

#[macro_use]
extern crate logging;
extern crate alloc;

use devices;
use frame_allocator;
use kalloc;
use panic_handler as _;

#[no_mangle]
fn main(hart_id: usize, device_tree: usize) {
    if hart_id != 0 {
        loop {}
    }

    // initialize kernel alloc module
    kalloc::init();

    // initialize logging module
    logging::init();

    // initialize device settings
    devices::init_device(device_tree);

    // initialize frame allocator
    frame_allocator::init();

    println!("Hello, world!");
}
