//! This module provides the `libc` types for Time (time management).
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/time.h>

use crate::types::TimeVal;

/// 定时器结构体，表示间隔和当前值（对应 C 语言中的 `struct itimerval`）
#[repr(C)]
#[derive(Clone, Debug, Default, Copy)]
pub struct ITimerVal {
    /// 重复触发的间隔时间（interval > 0 表示周期性定时器）
    pub interval: TimeVal,
    /// 当前倒计时的剩余时间（初始超时时长）
    pub value: TimeVal,
}
