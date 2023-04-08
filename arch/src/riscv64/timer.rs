use riscv::register::{sie, sstatus, time};

use crate::set_timer;

const CLOCK_FREQ: usize = 12500000;
const TICKS_PER_SEC: usize = 100;
#[allow(dead_code)]
const MSEC_PER_SEC: usize = 1000;

#[allow(dead_code)]
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

// 设置下一次时钟中断触发时间
pub fn set_next_timeout() {
    // 调用sbi设置定时器
    set_timer(time::read() + CLOCK_FREQ / TICKS_PER_SEC);
}

pub fn init() {
    unsafe {
        sie::set_stimer();
        sstatus::set_sie();
    }
    set_next_timeout();
    info!("initialize timer interrupt");
}
