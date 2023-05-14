#![no_std]

extern crate alloc;
pub mod interrupt;

pub fn current_nsec() -> usize {
    // devices::RTC_DEVICES.lock()[0].read() as usize
    // time_to_usec(get_time())
    arch::time_to_usec(arch::get_time()) * 1000
}
