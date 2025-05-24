//! This module provides the `libc` types for Signal.
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/powerpc/bits/signal.h>

pub use crate::arch::{MContext, SignalStackFlags, UContext, UStack};
use num_enum::TryFromPrimitive;

/// POSIX 标准、线程扩展与实时信号枚举定义（信号编号从 1 开始）
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/aarch64/bits/signal.h#L118>
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum SignalNum {
    /// 终端挂起（Hangup）
    HUP = 1,
    /// 交互式中断（Interrupt）
    INT,
    /// 退出（Quit）
    QUIT,
    /// 非法指令（Illegal instruction）
    ILL,
    /// 断点（Trace/breakpoint trap）
    TRAP,
    /// 异常终止（Abort）
    ABRT,
    /// 总线错误（Bus error）
    BUS,
    /// 浮点异常（Floating-point exception）
    FPE,
    /// 强制终止（Kill，不可被捕获或忽略）
    KILL,
    /// 用户定义信号 1
    USR1,
    /// 段错误（Segmentation fault）
    SEGV,
    /// 用户定义信号 2
    USR2,
    /// 管道破裂（Broken pipe）
    PIPE,
    /// 报警时钟（Alarm clock）
    ALRM,
    /// 终止请求（Termination）
    TERM,
    /// 协处理器堆栈故障（栈浮点错误，仅部分平台支持）
    STKFLT,
    /// 子进程终止或状态变化（Child）
    CHLD,
    /// 继续执行（Continue）
    CONT,
    /// 停止进程（不可忽略）
    STOP,
    /// 终端停止（来自 TTY 的 Ctrl+Z）
    TSTP,
    /// 后台读取控制终端
    TTIN,
    /// 后台写入控制终端
    TTOU,
    /// 紧急条件（Urgent socket）
    URG,
    /// 超过 CPU 时间限制
    XCPU,
    /// 超过文件大小限制
    XFSZ,
    /// 虚拟计时器到期（Virtual alarm）
    VTALRM,
    /// 性能分析计时器到期（Profiling alarm）
    PROF,
    /// 窗口大小改变（Window size change）
    WINCH,
    /// 异步 I/O（I/O now possible）
    IO,
    /// 电源失败（Power failure）
    PWR,
    /// 非法系统调用（Bad syscall）
    SYS,
    /// POSIX 线程：定时器信号
    TIMER,
    /// POSIX 线程：取消信号
    CANCEL,
    /// POSIX 线程：同步调用信号
    SYNCCALL,
    /// 实时信号 3（Real-time signal 3）
    RT3,
    /// 实时信号 4
    RT4,
    /// 实时信号 5
    RT5,
    /// 实时信号 6
    RT6,
    /// 实时信号 7
    RT7,
    /// 实时信号 8
    RT8,
    /// 实时信号 9
    RT9,
    /// 实时信号 10
    RT10,
    /// 实时信号 11
    RT11,
    /// 实时信号 12
    RT12,
    /// 实时信号 13
    RT13,
    /// 实时信号 14
    RT14,
    /// 实时信号 15
    RT15,
    /// 实时信号 16
    RT16,
    /// 实时信号 17
    RT17,
    /// 实时信号 18
    RT18,
    /// 实时信号 19
    RT19,
    /// 实时信号 20
    RT20,
    /// 实时信号 21
    RT21,
    /// 实时信号 22
    RT22,
    /// 实时信号 23
    RT23,
    /// 实时信号 24
    RT24,
    /// 实时信号 25
    RT25,
    /// 实时信号 26
    RT26,
    /// 实时信号 27
    RT27,
    /// 实时信号 28
    RT28,
    /// 实时信号 29
    RT29,
    /// 实时信号 30
    RT30,
    /// 实时信号 31
    RT31,
    /// 最大实时信号（Real-time signal max）
    RTMAX,
}

/// 实时信号（Real-time signals）的起始编号。
pub const REAL_TIME_SIGNAL_NUM: usize = 33;

impl SignalNum {
    /// 从数字构造 `SignalNum` 枚举，如果超出合法范围则返回 `None`。
    #[inline]
    pub fn from_num(num: usize) -> Option<SignalNum> {
        SignalNum::try_from(num as u8).ok()
    }

    /// 获取信号的编号（number）。
    pub const fn num(&self) -> usize {
        *self as usize
    }

    /// 判断是否为实时信号（Real-time Signal）。
    ///
    /// 实时信号从编号 [REAL_TIME_SIGNAL_NUM] 开始。
    pub const fn is_rt(&self) -> bool {
        *self as usize >= REAL_TIME_SIGNAL_NUM
    }

    /// 如果是实时信号，则返回其在实时信号表中的索引（从 1 开始）。
    ///
    /// 例如，编号为 [REAL_TIME_SIGNAL_NUM] 的信号返回 `1`。
    #[inline]
    pub fn real_time_index(&self) -> Option<usize> {
        self.is_rt().then(|| self.num() - 32)
    }

    /// 获取信号的位掩码（bit mask）。
    ///
    /// 信号编号从 1 开始，因此返回值为 `1 << (num - 1)`。
    pub const fn mask(&self) -> u64 {
        bit!(self.num() - 1)
    }
}
