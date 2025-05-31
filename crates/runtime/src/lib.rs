#![no_std]

extern crate alloc;

mod heap;

pub fn init() {
    heap::init();
}
