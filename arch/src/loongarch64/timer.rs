/// Returns the current clock time in hardware ticks.
use loongarch64::time::{get_timer_freq, Time};
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
