//! This module provides the `libc` types for Types.
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/alltypes.h.in>

use core::{cmp::Ordering, ops::Add};

/// IoVec structure
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/alltypes.h.in#L78>
#[repr(C)]
#[derive(Clone)]
pub struct IoVec {
    /// Base address of the buffer
    pub base: usize,
    /// Length of the buffer
    pub len: usize,
}

/// TimeVal structure
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/alltypes.h.in#L43>
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeVal {
    /// seconds, range in 0~999999999
    pub sec: usize,
    /// microseconds, range in 0~999999
    pub usec: usize,
}

impl Add for TimeVal {
    type Output = TimeVal;

    fn add(self, rhs: Self) -> Self::Output {
        let target = self.usec + rhs.usec;
        Self {
            sec: self.sec + rhs.sec + target / 1_000_000_000,
            usec: target % 1_000_000_000,
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
