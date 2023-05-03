use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use arch::get_time;
use devices::RTC_DEVICES;
use executor::{current_task, TMS};
use fs::TimeSpec;
use log::debug;

use crate::syscall::func::c2rust_ref;

use super::consts::LinuxError;

#[repr(C)]
pub struct TimeVal {
    sec: usize,  /* 秒 */
    usec: usize, /* 微秒, 范围在0~999999 */
}

pub async fn sys_gettimeofday(tv_ptr: usize, timezone_ptr: usize) -> Result<usize, LinuxError> {
    debug!(
        "sys_gettimeofday @ tv_ptr: {:#x}, timezone: {:#x}",
        tv_ptr, timezone_ptr
    );
    let ns = RTC_DEVICES.lock()[0].read() as usize;
    let ts = c2rust_ref(tv_ptr as *mut TimeVal);
    ts.sec = ns / 1_000_000_000;
    ts.usec = (ns % 1_000_000_000) / 1000;
    Ok(0)
}

pub async fn sys_nanosleep(req_ptr: usize, rem_ptr: usize) -> Result<usize, LinuxError> {
    debug!(
        "nano sleep @ req_ptr: {:#x}, rem_ptr: {:#x}",
        req_ptr, rem_ptr
    );
    let ns = RTC_DEVICES.lock()[0].read() as usize;
    let req = c2rust_ref(req_ptr as *mut TimeSpec);
    debug!(
        "sys_nanosleep @ req_ptr: {:#x}, req: {:#x}",
        req_ptr, rem_ptr
    );
    WaitUntilsec(ns + req.sec * 1_000_000_000 + req.nsec).await;
    if rem_ptr != 0 {
        let rem = c2rust_ref(rem_ptr as *mut TimeSpec);
        rem.nsec = 0;
        rem.sec = 0;
    }

    Ok(0)
}

pub async fn sys_times(tms_ptr: usize) -> Result<usize, LinuxError> {
    debug!("sys_times @ tms: {:#x}", tms_ptr);
    let tms = c2rust_ref(tms_ptr as *mut TMS);
    current_task()
        .as_user_task()
        .unwrap()
        .inner_map(|x| *tms = x.tms);
    Ok(get_time())
}

pub async fn sys_gettime(clock_id: usize, times_ptr: usize) -> Result<usize, LinuxError> {
    debug!(
        "sys_gettime @ clock_id: {}, times_ptr: {}",
        clock_id, times_ptr
    );

    let ts = c2rust_ref(times_ptr as *mut TimeSpec);
    let ns = RTC_DEVICES.lock()[0].read() as usize;

    ts.sec = ns / 1_000_000_000;
    ts.nsec = (ns % 1_000_000_000) / 1000;
    Ok(0)
}

pub struct WaitUntilsec(pub usize);

impl Future for WaitUntilsec {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ns = RTC_DEVICES.lock()[0].read() as usize;

        match ns > self.0 {
            true => Poll::Ready(()),
            false => Poll::Pending,
        }
    }
}

pub fn current_nsec() -> usize {
    RTC_DEVICES.lock()[0].read() as usize
}
