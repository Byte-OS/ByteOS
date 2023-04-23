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
struct TimeVal {
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
    let ns = RTC_DEVICES.lock()[0].read() as usize;
    let req = c2rust_ref(req_ptr as *mut TimeSpec);
    let rem = c2rust_ref(rem_ptr as *mut TimeSpec);
    debug!(
        "sys_nanosleep @ req_ptr: {:#x}, req: {:#x}",
        req_ptr, rem_ptr
    );
    WaitNsec(ns + req.sec * 1_000_000_000 + req.nsec).await;
    rem.nsec = 0;
    rem.sec = 0;

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

pub struct WaitNsec(usize);

impl Future for WaitNsec {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ns = RTC_DEVICES.lock()[0].read() as usize;

        match ns > self.0 {
            true => Poll::Ready(()),
            false => Poll::Pending,
        }
    }
}
