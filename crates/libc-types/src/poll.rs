//! This module provides the `libc` types for Poll (polling).
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/poll.h>

bitflags! {
    /// Poll 事件类型（类似于 epoll 的事件掩码）
    ///
    /// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/poll.h#L12>
    #[derive(Debug, Clone, PartialEq)]
    pub struct PollEvent: u16 {
        /// 无事件（默认值）
        const NONE = 0;
        /// 有数据可读
        const IN = 0x001;
        /// 有紧急数据可读（带外数据）
        const PRI = 0x002;
        /// 可写数据（缓冲区未满）
        const OUT = 0x004;
        /// 普通数据可读（等价于 POLLIN，用于区分优先级）
        const RDNORM = 0x040;
        /// 带外数据可读
        const RDBAND = 0x080;
        /// 普通数据可写（等价于 POLLOUT，用于区分优先级）
        const WRNORM = 0x100;
        /// 带外数据可写
        const WRBAND = 0x200;
        /// Linux 特有，可能与消息通知相关（通常不使用）
        const MSG = 0x400;
        /// 从 epoll 或 poll 实例中移除此文件描述符（Linux 特有）
        const REMOVE = 0x1000;
        /// 远端关闭（对端 shutdown write 或关闭 socket）
        const RDHUP = 0x2000;
        /// 错误事件（如写管道时接收端关闭）
        /// 不需要显式监听，默认总是报告
        const ERR = 0x008;
        /// 挂起事件（如对端关闭连接）
        /// 不需要显式监听，默认总是报告
        const HUP = 0x010;
        /// 无效的请求（如监听了一个无效的 fd）
        /// 不需要显式监听，默认总是报告
        const NVAL = 0x020;
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
/// 用于 poll 系统调用的文件描述符结构体
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/include/poll.h#L31>
pub struct PollFd {
    /// 文件描述符（File Descriptor），要监视的对象
    pub fd: u32,
    /// 期望监听的事件（如可读、可写等），由用户设置
    pub events: PollEvent,
    /// 实际发生的事件，由内核填写
    pub revents: PollEvent,
}
