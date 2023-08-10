use riscv::register::{sie, time};

use crate::set_timer;

pub use crate::riscv64::boards::CLOCK_FREQ;
const TICKS_PER_SEC: usize = 100;
#[allow(dead_code)]
const MSEC_PER_SEC: usize = 1000;
const USEC_PER_SEC: usize = 1000_000;
const NSEC_PER_SEC: usize = 1000_000_000;

#[allow(dead_code)]
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

#[inline]
pub fn get_time() -> usize {
    time::read()
}

#[inline]
pub fn time_to_usec(t: usize) -> usize {
    t / (CLOCK_FREQ / USEC_PER_SEC)
}

#[inline]
pub fn time_to_nsec(t: usize) -> usize {
    t * NSEC_PER_SEC / CLOCK_FREQ
}

// 设置下一次时钟中断触发时间
#[inline]
pub fn set_next_timeout() {
    // 调用sbi设置定时器
    set_timer(time::read() + CLOCK_FREQ / TICKS_PER_SEC);
}

pub fn init() {
    unsafe {
        sie::set_stimer();
        // sstatus::set_sie();
    }
    set_next_timeout();
    info!("initialize timer interrupt");
}
