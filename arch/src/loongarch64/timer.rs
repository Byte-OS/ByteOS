/// Returns the current clock time in hardware ticks.
use loongarch64::time::{get_timer_freq, Time};
use loongarch64::register::tcfg;
use loongarch64::register::ecfg::{self, LineBasedInterrupt};
use spin::Lazy;

// static mut FREQ: usize = 0;
static FREQ: Lazy<usize> = Lazy::new(|| get_timer_freq());

/// Returns the current clock time in hardware ticks.
#[inline]
pub fn get_time() -> usize {
    Time::read()
}

#[inline]
pub fn time_to_usec(ts: usize) -> usize {
    ts * 1000_000 / *FREQ
}

pub fn init_timer() {
    let ticks = ((*FREQ/1000) + 3) & !3;
    tcfg::set_periodic(true); // set timer to one-shot mode
    tcfg::set_init_val(ticks); // set timer initial value
    tcfg::set_en(true); // enable timer

    let inter = LineBasedInterrupt::TIMER
        | LineBasedInterrupt::SWI0
        | LineBasedInterrupt::SWI1
        | LineBasedInterrupt::HWI0;
    ecfg::set_lie(inter);
}
