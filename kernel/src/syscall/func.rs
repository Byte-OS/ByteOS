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

#[allow(dead_code)]
pub fn hexdump(data: &[u8]) {
    const PRELAND_WIDTH: usize = 70;
    println!("[kernel] {:-^1$}", " hexdump ", PRELAND_WIDTH);
    for offset in (0..data.len()).step_by(16) {
        print!("[kernel] ");
        for i in 0..16 {
            if offset + i < data.len() {
                print!("{:02x} ", data[offset + i]);
            } else {
                print!("{:02} ", "");
            }
        }

        print!("{:>6}", ' ');

        for i in 0..16 {
            if offset + i < data.len() {
                let c = data[offset + i];
                if c >= 0x20 && c <= 0x7e {
                    print!("{}", c as char);
                } else {
                    print!(".");
                }
            } else {
                print!("{:02} ", "");
            }
        }

        println!("");
    }
    println!("[kernel] {:-^1$}", " hexdump end ", PRELAND_WIDTH);
}
