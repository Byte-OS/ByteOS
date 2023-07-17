use core::{
    future::Future,
    ops::Add,
    pin::Pin,
    task::{Context, Poll},
};

use arch::{get_time, time_to_usec};
use executor::{current_task, current_user_task, select, AsyncTask, TMS};
use fs::TimeSpec;
pub use hal::current_nsec;
use hal::{ITimerVal, TimeVal};
use log::{debug, warn};

use crate::tasks::WaitHandleAbleSignal;

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
    debug!(
        "[task {}] sys_nanosleep @ req_ptr: {}, rem_ptr: {}",
        current_task().get_task_id(),
        req_ptr,
        rem_ptr
    );
    let ns = current_nsec();
    let req = req_ptr.get_mut();
    let task = current_user_task();
    debug!("nano sleep {} nseconds", req.sec * 1_000_000_000 + req.nsec);

    let res = match select(
        WaitHandleAbleSignal(task),
        WaitUntilsec(ns + req.sec * 1_000_000_000 + req.nsec),
    )
    .await
    {
        executor::Either::Right(_) => Ok(0),
        executor::Either::Left(_) => Err(LinuxError::EINTR),
    };
    if rem_ptr.is_valid() {
        *rem_ptr.get_mut() = Default::default();
    }
    res
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
    let task = current_user_task();
    debug!(
        "[task {}] sys_clock_gettime @ clock_id: {}, times_ptr: {}",
        task.get_task_id(),
        clock_id,
        times_ptr
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

#[inline]
pub async fn sys_clock_getres(
    clock_id: usize,
    times_ptr: UserRef<TimeSpec>,
) -> Result<usize, LinuxError> {
    debug!("clock_getres @ {} {:#x?}", clock_id, times_ptr);
    if times_ptr.is_valid() {
        *times_ptr.get_mut() = TimeSpec { sec: 0, nsec: 1 };
    }
    Ok(0)
    // sys_clock_gettime(clock_id, times_ptr).await
}
pub async fn sys_setitimer(
    which: usize,
    times_ptr: UserRef<ITimerVal>,
    old_timer_ptr: UserRef<ITimerVal>,
) -> Result<usize, LinuxError> {
    let task = current_user_task();
    debug!(
        "[task {}] sys_setitimer @ which: {} times_ptr: {} old_timer_ptr: {}",
        task.get_task_id(),
        which,
        times_ptr,
        old_timer_ptr
    );

    if which == 0 {
        let mut pcb = task.pcb.lock();
        if old_timer_ptr.is_valid() {
            // log::error!("old_timer: {:?}", old_timer_ptr.get_ref());
            *old_timer_ptr.get_mut() = pcb.timer[0].timer;
        }

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
        Ok(0)
    } else {
        Err(LinuxError::EPERM)
    }
}

pub async fn sys_clock_nanosleep(
    clock_id: usize,
    flags: usize,
    req_ptr: UserRef<TimeSpec>,
    rem_ptr: UserRef<TimeSpec>,
) -> Result<usize, LinuxError> {
    debug!(
        "[task {}] sys_clock_nanosleep @ clock_id: {}, flags: {:#x} req_ptr: {}, rem_ptr: {}",
        current_task().get_task_id(),
        clock_id,
        flags,
        req_ptr,
        rem_ptr
    );

    if flags == 1 {
        let req = req_ptr.get_mut();
        WaitUntilsec(req.sec * 1_000_000_000 + req.nsec).await;
        if rem_ptr.is_valid() {
            *rem_ptr.get_mut() = Default::default();
        }
    } else {
        let ns = current_nsec();
        let req = req_ptr.get_mut();
        debug!("nano sleep {} nseconds", req.sec * 1_000_000_000 + req.nsec);
        WaitUntilsec(ns + req.sec * 1_000_000_000 + req.nsec).await;
    }

    Ok(0)
}
