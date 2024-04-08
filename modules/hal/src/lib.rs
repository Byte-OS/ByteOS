#![no_std]
#![feature(decl_macro)]
#![feature(const_mut_refs)]
#![feature(const_option)]

use arch::time::Time;
use core::{cmp::Ordering, ops::Add};

extern crate alloc;
pub mod interrupt;

pub fn current_nsec() -> usize {
    // devices::RTC_DEVICES.lock()[0].read() as usize
    // arch::time_to_usec(arch::get_time()) * 1000
    Time::now().to_nsec()
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeVal {
    pub sec: usize,  /* 秒 */
    pub usec: usize, /* 微秒, 范围在0~999999 */
}

impl TimeVal {
    pub fn now() -> Self {
        let ns = current_nsec();
        Self {
            sec: ns / 1_000_000_000,
            usec: (ns % 1_000_000_000) / 1000,
        }
    }
}

impl Add for TimeVal {
    type Output = TimeVal;

    fn add(self, rhs: Self) -> Self::Output {
        let nsec = self.usec + rhs.usec;
        Self {
            sec: self.sec + rhs.sec + nsec / 1_000_000_000,
            usec: nsec % 1_000_000_000,
        }
    }
}

impl PartialOrd for TimeVal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.sec > other.sec {
            Some(Ordering::Greater)
        } else if self.sec < other.sec {
            Some(Ordering::Less)
        } else {
            if self.usec > other.usec {
                Some(Ordering::Greater)
            } else if self.usec < other.usec {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Equal)
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, Copy)]
pub struct ITimerVal {
    pub interval: TimeVal,
    pub value: TimeVal,
}
