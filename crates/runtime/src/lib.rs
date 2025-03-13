#![no_std]

#[macro_use]
extern crate alloc;

pub mod frame;
mod heap;

pub fn init() {
    heap::init();
}
