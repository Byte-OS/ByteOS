//! This module provides the `libc` types for Epoll (event polling).
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/epoll.h>

use num_enum::TryFromPrimitive;

use crate::poll::PollEvent;

/// 表示 epoll 事件的结构体（对应 Linux 的 `struct epoll_event`）
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/sys/epoll.h#L49>
/// TODO: 根据 data 的类型，可能需要使用不同的结构体来表示不同的事件, 对应 C 语言的 `union`
/// NOTE: 在 x86_64 架构会添加 `__attribute__ ((__packed__))`，以确保结构体的内存对齐
#[repr(C)]
#[derive(Clone, Debug)]
pub struct EpollEvent {
    /// 事件类型（如可读、可写等，使用 EpollEventType 表示）
    pub events: EpollEventType,
    /// 用户数据（如 fd 或标识符），epoll 不做解释
    pub data: u64,
}

bitflags! {
    /// Epoll 事件类型（类似 poll 的事件掩码）
    #[derive(Clone, Debug)]
    pub struct EpollEventType: u32 {
        /// 表示对应的文件描述符可读（包括普通数据和优先数据）
        const EPOLLIN = 0x001;
        /// 表示对应的文件描述符可写（低水位标记）
        const EPOLLOUT = 0x004;
        /// 文件描述符发生错误（error）
        const EPOLLERR = 0x008;
        /// 对端挂起或关闭连接（hang up）
        const EPOLLHUP = 0x010;
        /// 有高优先级数据可读（如带外数据）
        const EPOLLPRI = 0x002;
        /// 普通数据可读（normal read）
        const EPOLLRDNORM = 0x040;
        /// 带外数据可读（band read）
        const EPOLLRDBAND = 0x080;
        /// 普通数据可写（normal write）
        const EPOLLWRNORM = 0x100;
        /// 带外数据可写（band write）
        const EPOLLWRBAND = 0x200;
        /// 有系统消息可读（通常未使用）
        const EPOLLMSG = 0x400;
        /// 流被对端关闭，半关闭状态（对端调用 shutdown 写）
        const EPOLLRDHUP = 0x2000;
        /// 表示该监听是排他的（exclusive），用于防止多线程同时触发
        const EPOLLEXCLUSIVE = 0x1000_0000;
        /// 唤醒系统 suspend 状态（需要 CAP_BLOCK_SUSPEND 权限）
        const EPOLLWAKEUP = 0x2000_0000;
        /// 事件触发一次后就自动删除（one-shot 模式）
        const EPOLLONESHOT = 0x4000_0000;
        /// 边缘触发（Edge-Triggered）模式
        const EPOLLET = 0x8000_0000;
    }
}

impl EpollEventType {
    /// 将 EpollEventType 转换为 PollEvent
    pub fn to_poll(&self) -> PollEvent {
        PollEvent::from_bits_truncate(self.bits() as u16)
    }
}

#[repr(u8)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
/// `epoll_ctl` 操作类型，用于管理 epoll 实例中的监听目标（fd）。
pub enum EpollCtl {
    /// 添加一个新的监听目标到 epoll 实例中（epoll_ctl(epfd, EPOLL_CTL_ADD, fd, event)）
    ADD = 1,
    /// 从 epoll 实例中删除一个监听目标（epoll_ctl(epfd, EPOLL_CTL_DEL, fd, NULL)）
    DEL = 2,
    /// 修改已添加目标的监听事件（epoll_ctl(epfd, EPOLL_CTL_MOD, fd, event)）
    MOD = 3,
}
