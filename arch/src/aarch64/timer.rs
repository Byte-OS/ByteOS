#![allow(unused_imports)]

use aarch64_cpu::registers::{CNTFRQ_EL0, CNTPCT_EL0, CNTP_CTL_EL0, CNTP_TVAL_EL0};
use tock_registers::interfaces::{Readable, Writeable};

/// Returns the current clock time in hardware ticks.
#[inline]
pub fn get_time() -> usize {
    CNTPCT_EL0.get() as _
}

#[inline]
pub fn time_to_usec(ts: usize) -> usize {
    ts * 1000_000 / CNTFRQ_EL0.get() as usize
}

pub fn set_next_timer() {
    CNTP_TVAL_EL0.set(CNTFRQ_EL0.get() / 1000);
}

pub fn init() {
    let freq = CNTFRQ_EL0.get();
    debug!("freq: {}", freq);
    CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
    CNTP_TVAL_EL0.set(0);
    super::gic::set_enable(super::gic::TIMER_IRQ_NUM, true);
    set_next_timer();
}
