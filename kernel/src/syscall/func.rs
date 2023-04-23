use core::ffi::CStr;

use devices::RTC_DEVICES;
use fs::TimeSpec;

pub fn timespc_now() -> TimeSpec {
    let ns = RTC_DEVICES.lock()[0].read() as usize;

    TimeSpec {
        sec: ns / 1_000_000_000,
        nsec: (ns % 1_000_000_000) / 1000,
    }
}

pub fn c2rust_list<T>(ptr: *mut T, valid: fn(T) -> bool) -> &'static mut [T] {
    unsafe {
        let mut len = 0;
        if !ptr.is_null() {
            loop {
                if !valid(ptr.add(len).read()) {
                    break;
                }
                len += 1;
            }
        }
        core::slice::from_raw_parts_mut(ptr, len)
    }
}

pub fn c2rust_buffer<T>(ptr: *mut T, count: usize) -> &'static mut [T] {
    unsafe { core::slice::from_raw_parts_mut(ptr, count) }
}

pub fn c2rust_str(ptr: *const i8) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap()
}

pub fn c2rust_ref<T>(ptr: *mut T) -> &'static mut T {
    unsafe { ptr.as_mut().unwrap() }
}
