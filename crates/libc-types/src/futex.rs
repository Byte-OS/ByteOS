//! This module provides the `libc` types for FUTEX (fast user-space mutex).
use num_enum::TryFromPrimitive;

/// Futex 操作类型枚举
///
/// MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/src/internal/futex.h#L4>
#[derive(Debug, TryFromPrimitive)]
#[repr(usize)]
pub enum FutexFlags {
    /// 等待操作，线程阻塞直到被唤醒
    Wait = 0,
    /// 唤醒等待的线程
    Wake = 1,
    /// 使用文件描述符的 Futex 操作（较少用）
    Fd = 2,
    /// 将等待队列中的线程重新排队到另一个 Futex
    Requeue = 3,
    /// 带比较操作的重新排队，只有在值匹配时才执行排队
    CmpRequeue = 4,
    /// 执行复杂的唤醒和重新排队组合操作
    WakeOp = 5,
    /// 获取 Priority Inheritance 锁
    LockPi = 6,
    /// 释放 Priority Inheritance 锁
    UnlockPi = 7,
    /// 尝试获取 Priority Inheritance 锁（非阻塞）
    TrylockPi = 8,
    /// 等待指定的位集合（bitset），类似于 Wait，但支持位掩码
    WaitBitset = 9,
}

/// 标志：表示 futex 是私有的，只在同一进程内使用（性能更好）
/// 相当于 FUTEX_PRIVATE_FLAG，避免跨进程同步开销
pub const FUTEX_PRIVATE: usize = 128;

/// 标志：使用系统实时时钟（CLOCK_REALTIME）作为超时基准
/// 默认 futex 超时使用的是 CLOCK_MONOTONIC，设置此标志改为使用实时时钟
pub const FUTEX_CLOCK_REALTIME: usize = 256;
