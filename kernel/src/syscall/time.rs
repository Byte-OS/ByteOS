use core::{
    future::Future,
    ops::Add,
    pin::Pin,
    task::{Context, Poll},
};

use arch::{get_time, time_to_usec};
use executor::{current_user_task, TMS};
use fs::TimeSpec;
pub use hal::current_nsec;
use hal::{ITimerVal, TimeVal};
use log::{debug, warn};

use super::consts::{LinuxError, UserRef};

pub async fn sys_gettimeofday(
    tv_ptr: UserRef<TimeVal>,
    timezone_ptr: usize,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_gettimeofday @ tv_ptr: {}, timezone: {:#x}",
        tv_ptr, timezone_ptr
    );
    *tv_ptr.get_mut() = TimeVal::now();
    Ok(0)
}

pub async fn sys_nanosleep(
    req_ptr: UserRef<TimeSpec>,
    rem_ptr: UserRef<TimeSpec>,
) -> Result<usize, LinuxError> {
    debug!("sys_nanosleep @ req_ptr: {}, rem_ptr: {}", req_ptr, rem_ptr);
    let ns = current_nsec();
    let req = req_ptr.get_mut();
    WaitUntilsec(ns + req.sec * 1_000_000_000 + req.nsec).await;
    if rem_ptr.is_valid() {
        *rem_ptr.get_mut() = Default::default();
    }
    Ok(0)
}

pub async fn sys_times(tms_ptr: UserRef<TMS>) -> Result<usize, LinuxError> {
    debug!("sys_times @ tms: {}", tms_ptr);
    current_user_task().inner_map(|x| *tms_ptr.get_mut() = x.tms);
    Ok(get_time())
}

pub async fn sys_clock_gettime(
    clock_id: usize,
    times_ptr: UserRef<TimeSpec>,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_clock_gettime @ clock_id: {}, times_ptr: {}",
        clock_id, times_ptr
    );

    let ns = match clock_id {
        0 => current_nsec(),                  // CLOCK_REALTIME
        1 => time_to_usec(get_time()) * 1000, // CLOCK_MONOTONIC
        2 => {
            warn!("CLOCK_PROCESS_CPUTIME_ID not implemented");
            0
        }
        3 => {
            warn!("CLOCK_THREAD_CPUTIME_ID not implemented");
            0
        }
        _ => return Err(LinuxError::EINVAL),
    };

    *times_ptr.get_mut() = TimeSpec {
        sec: ns / 1_000_000_000,
        nsec: ns % 1_000_000_000,
    };
    Ok(0)
}

pub struct WaitUntilsec(pub usize);

impl Future for WaitUntilsec {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ns = current_nsec();

        match ns > self.0 {
            true => Poll::Ready(()),
            false => Poll::Pending,
        }
    }
}

#[allow(dead_code)]
pub fn wait_ms(ms: usize) -> WaitUntilsec {
    WaitUntilsec(current_nsec() + ms * 0x1000_0000)
}

pub async fn sys_setitimer(
    which: usize,
    times_ptr: UserRef<ITimerVal>,
    old_timer_ptr: UserRef<ITimerVal>,
) -> Result<usize, LinuxError> {
    debug!(
        "sys_setitimer @ which: {} times_ptr: {} old_timer_ptr: {}",
        which, times_ptr, old_timer_ptr
    );

    if which == 0 {
        let task = current_user_task();
        let mut pcb = task.pcb.lock();
        if times_ptr.is_valid() {
            let new_timer = times_ptr.get_ref();
            // log::error!("timer: {:?}", times_ptr.get_ref());
            pcb.timer[0].timer = *new_timer;
            pcb.timer[0].next = TimeVal::now().add(pcb.timer[0].timer.value);
            if new_timer.value.sec == 0 && new_timer.value.usec == 0 {
                pcb.timer[0].next = Default::default();
                pcb.timer[0].last = Default::default();
            }
            // log::error!("process timer: {:?}", pcb.timer[0]);
        }
        if old_timer_ptr.is_valid() {
            // log::error!("old_timer: {:?}", old_timer_ptr.get_ref());
            *old_timer_ptr.get_mut() = pcb.timer[0].timer;
        }
        Ok(0)
    } else {
        Err(LinuxError::EPERM)
    }
}
