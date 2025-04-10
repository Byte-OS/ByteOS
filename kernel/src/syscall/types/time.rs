use core::{cmp::Ordering, ops::Add};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeVal {
    pub sec: usize,  /* 秒 */
    pub usec: usize, /* 微秒, 范围在0~999999 */
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

// tms_utime记录的是进程执行用户代码的时间.
// tms_stime记录的是进程执行内核代码的时间.
// tms_cutime记录的是子进程执行用户代码的时间.
// tms_ustime记录的是子进程执行内核代码的时间.
#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub struct TMS {
    pub utime: u64,
    pub stime: u64,
    pub cutime: u64,
    pub cstime: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessTimer {
    pub timer: ITimerVal,
    pub next: TimeVal,
    pub last: TimeVal,
}
