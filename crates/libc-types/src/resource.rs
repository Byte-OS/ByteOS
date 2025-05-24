//! This module provides the `libc` types for Resource (system resource management).
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/resource.h>

use crate::types::TimeVal;

/// 资源限制结构体（对应 C 的 `struct rlimit`）
/// 用于描述进程对某种资源的当前限制和最大限制
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Rlimit {
    /// 当前资源软限制（soft limit），即实际生效的限制值
    /// 进程可以在不超过 `max` 的情况下修改它
    pub curr: usize,
    /// 最大资源限制（hard limit），软限制不能超过该值
    /// 只有具有特权的进程才能提升此值
    pub max: usize,
}

/// 资源使用情况结构体（对应 C 的 `struct rusage`）
/// 记录进程或线程的时间和资源消耗信息
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/resource.h#L27>
#[repr(C)]
pub struct Rusage {
    /// 用户态运行时间（单位：秒 + 微秒）
    pub utime: TimeVal,
    /// 内核态运行时间（单位：秒 + 微秒）
    pub stime: TimeVal,
    /// 最大常驻集大小（单位：KB），即内存占用峰值
    pub maxrss: i64,
    /// 索引页的使用数量（已废弃）
    pub ixrss: i64,
    /// 数据段内存使用量（已废弃）
    pub idrss: i64,
    /// 堆栈段内存使用量（已废弃）
    pub isrss: i64,
    /// 页面缺页异常数（软缺页，不涉及磁盘 IO）
    pub minflt: i64,
    /// 主缺页异常数（硬缺页，需要从磁盘读取页面）
    pub majflt: i64,
    /// 发生的交换（swap）次数
    pub nswap: i64,
    /// 输入操作（块设备读取）的次数
    pub inblock: i64,
    /// 输出操作（块设备写入）的次数
    pub oublock: i64,
    /// 发送的 IPC 消息数（已废弃）
    pub msgsnd: i64,
    /// 接收的 IPC 消息数（已废弃）
    pub msgrcv: i64,
    /// 捕获的信号数量
    pub nsignals: i64,
    /// 自愿上下文切换次数（如等待锁）
    pub nvcsw: i64,
    /// 非自愿上下文切换次数（被内核抢占）
    pub nivcsw: i64,
}
