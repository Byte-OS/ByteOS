use core::{
    future::Future,
    ops::Add,
    pin::Pin,
    task::{Context, Poll},
};

use executor::select;
use fs::TimeSpec;
pub use hal::current_nsec;
use hal::{ITimerVal, TimeVal};
use log::{debug, warn};
use polyhal::time::Time;

use crate::{
    tasks::{WaitHandleAbleSignal, TMS},
    user::UserTaskContainer,
};

use super::{
    consts::{LinuxError, UserRef},
    SysResult,
};
impl UserTaskContainer {
    pub async fn sys_gettimeofday(
        &self,
        tv_ptr: UserRef<TimeVal>,
        timezone_ptr: usize,
    ) -> SysResult {
        debug!(
            "sys_gettimeofday @ tv_ptr: {}, timezone: {:#x}",
            tv_ptr, timezone_ptr
        );
        *tv_ptr.get_mut() = TimeVal::now();
        Ok(0)
    }

    pub async fn sys_nanosleep(
        &self,
        req_ptr: UserRef<TimeSpec>,
        rem_ptr: UserRef<TimeSpec>,
    ) -> SysResult {
        debug!(
            "[task {}] sys_nanosleep @ req_ptr: {}, rem_ptr: {}",
            self.tid, req_ptr, rem_ptr
        );
        let ns = current_nsec();
        let req = req_ptr.get_mut();
        debug!("nano sleep {} nseconds", req.sec * 1_000_000_000 + req.nsec);

        let res = match select(
            WaitHandleAbleSignal(self.task.clone()),
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

    pub async fn sys_times(&self, tms_ptr: UserRef<TMS>) -> SysResult {
        debug!("sys_times @ tms: {}", tms_ptr);
        self.task.inner_map(|x| *tms_ptr.get_mut() = x.tms);
        Ok(Time::now().raw())
    }

    pub async fn sys_clock_gettime(
        &self,
        clock_id: usize,
        times_ptr: UserRef<TimeSpec>,
    ) -> SysResult {
        debug!(
            "[task {}] sys_clock_gettime @ clock_id: {}, times_ptr: {}",
            self.tid, clock_id, times_ptr
        );

        let ns = match clock_id {
            0 => current_nsec(),        // CLOCK_REALTIME
            1 => Time::now().to_nsec(), // CLOCK_MONOTONIC
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

    #[inline]
    pub async fn sys_clock_getres(
        &self,
        clock_id: usize,
        times_ptr: UserRef<TimeSpec>,
    ) -> SysResult {
        debug!("clock_getres @ {} {:#x?}", clock_id, times_ptr);
        if times_ptr.is_valid() {
            *times_ptr.get_mut() = TimeSpec { sec: 0, nsec: 1 };
        }
        Ok(0)
    }
    pub async fn sys_setitimer(
        &self,
        which: usize,
        times_ptr: UserRef<ITimerVal>,
        old_timer_ptr: UserRef<ITimerVal>,
    ) -> SysResult {
        debug!(
            "[task {}] sys_setitimer @ which: {} times_ptr: {} old_timer_ptr: {}",
            self.tid, which, times_ptr, old_timer_ptr
        );

        if which == 0 {
            let mut pcb = self.task.pcb.lock();
            if old_timer_ptr.is_valid() {
                *old_timer_ptr.get_mut() = pcb.timer[0].timer;
            }

            if times_ptr.is_valid() {
                let new_timer = times_ptr.get_ref();
                pcb.timer[0].timer = *new_timer;
                pcb.timer[0].next = TimeVal::now().add(pcb.timer[0].timer.value);
                if new_timer.value.sec == 0 && new_timer.value.usec == 0 {
                    pcb.timer[0].next = Default::default();
                    pcb.timer[0].last = Default::default();
                }
            }
            Ok(0)
        } else {
            Err(LinuxError::EPERM)
        }
    }

    pub async fn sys_clock_nanosleep(
        &self,
        clock_id: usize,
        flags: usize,
        req_ptr: UserRef<TimeSpec>,
        rem_ptr: UserRef<TimeSpec>,
    ) -> SysResult {
        debug!(
            "[task {}] sys_clock_nanosleep @ clock_id: {}, flags: {:#x} req_ptr: {}, rem_ptr: {}",
            self.tid, clock_id, flags, req_ptr, rem_ptr
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
