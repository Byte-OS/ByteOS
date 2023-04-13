#![no_main]
#![no_std]
#![feature(exclusive_range_pattern)]

#[macro_use]
extern crate logging;
#[macro_use]
extern crate alloc;

mod syscall;
mod tasks;

use devices;
use frame_allocator;
use hal;
use kalloc;
use panic_handler as _;

#[no_mangle]
fn main(hart_id: usize, device_tree: usize) {
    // if hart_id != 0 {
    //     loop {}
    // }

    let str = include_str!("banner.txt");
    println!("{}", str);

    // initialize logging module
    logging::init();

    // initialize interrupt
    hal::interrupt::init();

    // print boot info
    info!("booting at kernel {}", hart_id);

    // initialize kernel alloc module
    kalloc::init();

    // initialize device settings
    devices::init_device(device_tree);

    // initialize frame allocator
    frame_allocator::init();

    // get devices and init
    devices::prepare_devices();

    // initialize filesystem
    fs::init();

    // init kernel threads and async executor
    tasks::init();

    println!("Hello, world!");
}
