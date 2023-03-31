#![no_main]
#![no_std]

#[macro_use]
extern crate logging;
extern crate alloc;
use kalloc;
use panic_handler as _;

#[no_mangle]
fn main(hart_id: usize) {
    if hart_id != 0 {
        loop {}
    }

    // initialize kernel alloc module
    kalloc::init();

    // initialize logging module
    logging::init();

    println!("Hello, world!");
}
