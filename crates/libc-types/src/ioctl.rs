//! This module provides the `libc` types for IOCTL.
//!
//! MUSL: <https://github.com/bminor/musl/blob/c47ad25ea3b484e10326f933e927c0bc8cded3da/arch/generic/bits/ioctl.h#L11>
//! TODO: Check ioctl command for multi architectures.

use num_enum::TryFromPrimitive;

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u32)]
/// Teletype 设备相关 ioctl 命令，用于控制终端（如串口、TTY）行为。
pub enum TermIoctlCmd {
    // 用于 struct termios
    /// 获取当前串口设置（termios 结构体）
    TCGETS = 0x5401,
    /// 立即设置串口配置（termios 结构体）
    TCSETS = 0x5402,
    /// 等待输出缓冲区刷新后再设置串口配置
    TCSETSW = 0x5403,
    /// 刷新输入输出缓冲区后设置串口配置
    TCSETSF = 0x5404,

    // 用于 struct termio（旧接口）
    /// 获取当前串口设置（termio 结构体）
    TCGETA = 0x5405,
    /// 立即设置串口配置（termio 结构体）
    TCSETA = 0x5406,
    /// 等待输出缓冲区刷新后设置串口配置
    TCSETAW = 0x5407,
    /// 刷新输入输出缓冲区后设置串口配置
    TCSETAF = 0x5408,

    /// 获取当前终端的前台进程组 ID
    TIOCGPGRP = 0x540F,
    /// 设置当前终端的前台进程组 ID
    TIOCSPGRP = 0x5410,

    /// 获取终端窗口大小（通常与 struct winsize 搭配）
    TIOCGWINSZ = 0x5413,
    /// 设置终端窗口大小
    TIOCSWINSZ = 0x5414,

    /// 取消 `close-on-exec` 标志（在 `exec` 执行时文件描述符不会自动关闭）
    FIONCLEX = 0x5450,
    /// 设置 `close-on-exec` 标志（在 `exec` 执行时自动关闭文件描述符）
    FIOCLEX = 0x5451,

    /// 设置非阻塞 I/O（rustc 编译器也会用这个 ioctl 命令控制 pipe 行为）
    FIONBIO = 0x5421,

    /// 获取 RTC（实时时钟）的当前时间（用于 RTC 设备）
    RTCRDTIME = 0x80247009,
}
