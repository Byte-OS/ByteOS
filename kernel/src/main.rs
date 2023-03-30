#![no_main]
#![no_std]

#[macro_use]
extern crate logging;
use panic_handler as _;

#[no_mangle]
extern "Rust" fn main(hart_id: usize) {
    if hart_id != 0 {
        loop {}
    }

    // initialize logging module
    logging::init();
    println!("Hello, world!");
}
