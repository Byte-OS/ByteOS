use libc_types::types::{TimeSpec, TimeVal};
use polyhal::Time;

#[inline]
pub fn current_nsec() -> usize {
    // devices::RTC_DEVICES.lock()[0].read() as usize
    // arch::time_to_usec(arch::get_time()) * 1000
    Time::now().to_nsec()
}

#[inline]
pub fn current_timeval() -> TimeVal {
    let ns = current_nsec();
    TimeVal {
        sec: ns / 1_000_000_000,
        usec: (ns % 1_000_000_000) / 1000,
    }
}

#[inline]
pub fn current_timespec() -> TimeSpec {
    let ns = current_nsec();

    TimeSpec {
        sec: ns / 1_000_000_000,
        nsec: (ns % 1_000_000_000),
    }
}
